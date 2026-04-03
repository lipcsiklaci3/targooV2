use libsql::{Connection, params};
use std::sync::Arc;
use csv::ReaderBuilder;
use calamine::{Reader, Xlsx};
use std::fs;
use serde_json::json;
use crate::ingest::{normalize_header, extract_unit};
use crate::physics;

pub async fn process_job_task(conn: Arc<Connection>, job_id: String, files: Vec<String>) {
    let mut processed_rows = 0;

    for file_name in &files {
        let file_path = format!("temp/{}/{}", job_id, file_name);
        
        if file_name.ends_with(".csv") {
            if let Ok(data) = fs::read(&file_path) {
                let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(&data[..]);
                let headers: Vec<String> = rdr.headers().unwrap().iter().map(|h| h.to_string()).collect();

                // Prefetch some samples for HITL context
                let mut samples_cache: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

                for result in rdr.records() {
                    if let Ok(record) = result {
                        processed_rows += 1;
                        for (col_idx, value) in record.iter().enumerate() {
                            let raw_header = &headers[col_idx];
                            
                            // Collect samples for every header just in case
                            let samples = samples_cache.entry(raw_header.clone()).or_insert(Vec::new());
                            if samples.len() < 5 {
                                samples.push(value.to_string());
                            }

                            process_cell(&conn, &job_id, file_name, processed_rows, raw_header, value, samples).await;
                        }
                        update_progress(&conn, &job_id, processed_rows).await;
                    }
                }
            }
        } else if file_name.ends_with(".xlsx") {
            if let Ok(mut excel) = calamine::open_workbook::<Xlsx<_>, _>(&file_path) {
                for sheet_name in excel.sheet_names().to_owned() {
                    if let Ok(range) = excel.worksheet_range(&sheet_name) {
                        let mut iter = range.rows();
                        let headers: Vec<String> = match iter.next() {
                            Some(h) => h.iter().map(|c| c.to_string()).collect(),
                            None => continue,
                        };

                        let mut samples_cache: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

                        for row in iter {
                            processed_rows += 1;
                            for (col_idx, cell) in row.iter().enumerate() {
                                let raw_header = &headers[col_idx];
                                let value = cell.to_string();

                                let samples = samples_cache.entry(raw_header.clone()).or_insert(Vec::new());
                                if samples.len() < 5 {
                                    samples.push(value.clone());
                                }

                                process_cell(&conn, &job_id, file_name, processed_rows, raw_header, &value, samples).await;
                            }
                            update_progress(&conn, &job_id, processed_rows).await;
                        }
                    }
                }
            }
        }
    }

    let _ = conn.execute(
        "UPDATE job_registry SET status = 'COMPLETE' WHERE job_id = ?",
        params![job_id.clone()]
    ).await;
    println!("[SYSTEM] Job '{}' processing complete.", job_id);
    
    let _ = fs::remove_dir_all(format!("temp/{}", job_id));
}

async fn process_cell(conn: &Connection, job_id: &str, file_name: &str, row_num: i64, raw_header: &str, value: &str, samples: &[String]) {
    let norm_header = normalize_header(raw_header);
    let input_unit = extract_unit(raw_header);

    let mut mapping = lookup_mapping(conn, &norm_header).await;

    if mapping.is_none() {
        // Trigger HITL with high context (SAD v2.0 requirement)
        let sample_vals_json = json!(samples).to_string();
        let _ = conn.execute(
            "INSERT INTO hitl_queue (job_id, raw_header, sample_values) VALUES (?, ?, ?)",
            params![job_id.to_string(), raw_header.to_string(), sample_vals_json]
        ).await;

        wait_for_hitl(conn, job_id).await;
        
        mapping = lookup_mapping(conn, &norm_header).await;
    }

    if let Some((jur, cat)) = mapping {
        match physics::calculate_refined_emissions(conn, job_id, row_num, value, &input_unit, &cat).await {
            Ok(res) => {
                insert_ledger(conn, job_id, &jur, file_name, row_num, raw_header, value, &cat, res.canonical_value, &res.canonical_unit, &res.factor_source, res.tco2e, &res.row_sha256).await;
            }
            Err(e) => {
                let _ = conn.execute(
                    "INSERT INTO esg_ledger (job_id, jurisdiction, source_filename, source_row_number, source_header_raw, raw_value, status, row_sha256, factor_source) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![job_id.to_string(), jur, file_name.to_string(), row_num, raw_header.to_string(), value.to_string(), "quarantined", format!("ERR-{}-{}", job_id, uuid::Uuid::new_v4()), e.to_string()]
                ).await;
            }
        }
    }
}

async fn lookup_mapping(conn: &Connection, norm_header: &str) -> Option<(String, String)> {
    let mut rows = conn.query("SELECT jurisdiction, target_category, keywords FROM mapping_dictionary", ()).await.unwrap();
    while let Some(row) = rows.next().await.unwrap() {
        let jur: String = row.get(0).unwrap();
        let cat: String = row.get(1).unwrap();
        let kw_json: String = row.get(2).unwrap();
        let kws: Vec<String> = serde_json::from_str(&kw_json).unwrap_or_default();
        
        for kw in kws {
            let norm_kw = kw.to_lowercase().replace('_', " ");
            if norm_header.contains(&norm_kw) || norm_kw.contains(norm_header) {
                return Some((jur, cat));
            }
        }
    }
    None
}

async fn wait_for_hitl(conn: &Connection, job_id: &str) {
    println!("[PAUSE] Job '{}' is waiting for HITL resolution.", job_id);
    let _ = conn.execute("UPDATE job_registry SET status = 'PAUSED_HITL' WHERE job_id = ?", params![job_id.to_string()]).await;

    loop {
        let mut rows = conn.query("SELECT status FROM job_registry WHERE job_id = ?", params![job_id.to_string()]).await.unwrap();
        if let Ok(Some(row)) = rows.next().await {
            let status: String = row.get(0).unwrap();
            if status == "PROCESSING" {
                println!("[RESUME] Job '{}' resumed.", job_id);
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

async fn update_progress(conn: &Connection, job_id: &str, processed: i64) {
    let _ = conn.execute(
        "UPDATE job_registry SET processed_rows = ? WHERE job_id = ?",
        params![processed, job_id.to_string()]
    ).await;
}

async fn insert_ledger(
    conn: &Connection, job_id: &str, jur: &str, file_name: &str, row_num: i64, 
    raw_header: &str, value: &str, cat: &str, can_val: f64, can_unit: &str,
    source: &str, tco2e: f64, sha: &str
) {
    let _ = conn.execute(
        "INSERT INTO esg_ledger (
            job_id, source_filename, source_row_number, source_header_raw, 
            raw_value, canonical_value, canonical_unit, ef_factor_code, 
            tco2e, row_sha256, status, factor_source, jurisdiction
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            job_id.to_string(), file_name.to_string(), row_num, raw_header.to_string(), value.to_string(), can_val, 
            can_unit.to_string(), cat.to_string(), tco2e, sha.to_string(), "clean", source.to_string(), jur.to_string()
        ]
    ).await;
}
