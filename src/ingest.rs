use axum::{extract::{Multipart, State}, Json};
use serde_json::{json, Value};
use libsql::{Connection, params};
use std::sync::Arc;
use uuid::Uuid;
use std::fs;
use std::io::Write;
use regex::Regex;
use crate::processor;

pub fn normalize_header(header: &str) -> String {
    let mut normalized = header.to_lowercase();
    let bracket_re = Regex::new(r"[\(\[].*?[\)\]]").unwrap();
    normalized = bracket_re.replace_all(&normalized, "").to_string();
    let special_re = Regex::new(r"[^a-z0-9\s]").unwrap();
    normalized = special_re.replace_all(&normalized, " ").to_string();
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn extract_unit(header: &str) -> String {
    if let Some(start) = header.find('(') {
        if let Some(end) = header.find(')') {
            return header[start + 1..end].trim().to_lowercase();
        }
    }
    if let Some(start) = header.find('[') {
        if let Some(end) = header.find(']') {
            return header[start + 1..end].trim().to_lowercase();
        }
    }
    "unknown".to_string()
}

pub async fn upload_handler(
    State(conn): State<Arc<Connection>>,
    mut multipart: Multipart,
) -> Json<Value> {
    let job_id = Uuid::new_v4().to_string();
    let mut saved_files = Vec::new();

    // Create temp directory
    let temp_dir = format!("temp/{}", job_id);
    fs::create_dir_all(&temp_dir).unwrap();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.unwrap();
        
        let file_path = format!("{}/{}", temp_dir, file_name);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(&data).unwrap();
        
        saved_files.push(file_name);
    }

    // Register Job
    let _ = conn.execute(
        "INSERT INTO job_registry (job_id, status) VALUES (?, 'PROCESSING')",
        params![job_id.clone()]
    ).await;

    println!("[SYSTEM] Job '{}' registered. Starting background worker.", job_id);

    // Spawn background task
    let background_conn = Arc::clone(&conn);
    let background_job_id = job_id.clone();
    let background_files = saved_files.clone();
    
    tokio::spawn(async move {
        processor::process_job_task(background_conn, background_job_id, background_files).await;
    });

    Json(json!({
        "status": "UPLOAD_SUCCESS",
        "job_id": job_id,
        "files_received": saved_files.len()
    }))
}
