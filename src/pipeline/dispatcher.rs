// Result distribution and storage coordination
use crate::models::MappingResult;
use crate::pipeline::mapper::Mapper;
use std::io::Cursor;
use calamine::{Reader, Xlsx, Data};
use csv::ReaderBuilder;
use serde_json::Value;

pub enum FileType { Csv, Xlsx, Json, Xml, Unknown }

pub struct RawRow {
    pub headers: Vec<String>,
    pub values: Vec<String>,
}

pub struct DispatchResult {
    pub clean: Vec<MappingResult>,
    pub best_effort: Vec<MappingResult>,
    pub quarantined: Vec<MappingResult>,
    pub total_rows: usize,
    pub total_tco2e: f64,
    pub processing_errors: Vec<String>,
}

fn detect_file_type(filename: &str) -> FileType {
    let ext = filename.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "csv" => FileType::Csv,
        "xlsx" | "xls" => FileType::Xlsx,
        "json" => FileType::Json,
        "xml" => FileType::Xml,
        _ => FileType::Unknown,
    }
}

fn parse_csv(data: &[u8]) -> Result<Vec<RawRow>, String> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(data);

    let headers: Vec<String> = rdr.headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| e.to_string())?;
        if record.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        rows.push(RawRow {
            headers: headers.clone(),
            values: record.iter().map(|s| s.to_string()).collect(),
        });
    }
    Ok(rows)
}

fn parse_xlsx(data: &[u8]) -> Result<Vec<RawRow>, String> {
    let cursor = Cursor::new(data);
    let mut excel: Xlsx<_> = Xlsx::new(cursor).map_err(|e| e.to_string())?;
    
    let sheet_name = excel.sheet_names()
        .first()
        .cloned()
        .ok_or("No sheets found in XLSX")?;
    
    let range = excel.worksheet_range(&sheet_name)
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    let mut iter = range.rows();
    
    let headers: Vec<String> = match iter.next() {
        Some(row) => row.iter().map(|c| c.to_string()).collect(),
        None => return Ok(rows),
    };

    for row in iter {
        if row.iter().all(|c| matches!(c, Data::Empty)) {
            continue;
        }
        let values: Vec<String> = row.iter().map(|c| {
            match c {
                Data::String(s) => s.clone(),
                Data::Float(f) => f.to_string(),
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::DateTime(dt) => dt.to_string(),
                _ => "".to_string(),
            }
        }).collect();

        rows.push(RawRow {
            headers: headers.clone(),
            values,
        });
    }
    Ok(rows)
}

fn parse_json(data: &[u8]) -> Result<Vec<RawRow>, String> {
    let v: Value = serde_json::from_slice(data).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("JSON must be an array of objects")?;
    
    let mut rows = Vec::new();
    for item in arr {
        let obj = item.as_object().ok_or("JSON array must contain objects")?;
        let mut headers = Vec::new();
        let mut values = Vec::new();
        for (k, val) in obj {
            headers.push(k.clone());
            values.push(match val {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => "".to_string(),
            });
        }
        rows.push(RawRow { headers, values });
    }
    Ok(rows)
}

fn extract_unit_hint(header: &str) -> String {
    // Look for (unit) or "in unit"
    if let Some(start) = header.find('(') {
        if let Some(end) = header.find(')') {
            if end > start {
                return header[start + 1..end].trim().to_string();
            }
        }
    }
    
    let parts: Vec<&str> = header.split_whitespace().collect();
    for i in 0..parts.len() {
        if parts[i].to_lowercase() == "in" && i + 1 < parts.len() {
            return parts[i + 1].trim().to_string();
        }
    }
    
    "".to_string()
}

fn process_rows(rows: Vec<RawRow>) -> DispatchResult {
    let mut clean = Vec::new();
    let mut best_effort = Vec::new();
    let mut quarantined = Vec::new();
    let mut total_rows = 0;
    let mut total_tco2e = 0.0;

    for row in rows {
        total_rows += 1;
        for (header, value) in row.headers.iter().zip(row.values.iter()) {
            let unit_hint = extract_unit_hint(header);
            let res = Mapper::process(header, value, &unit_hint);
            
            match res.status.as_str() {
                "clean" => {
                    total_tco2e += res.tco2e;
                    clean.push(res);
                }
                "best_effort" => {
                    best_effort.push(res);
                }
                _ => {
                    quarantined.push(res);
                }
            }
        }
    }

    DispatchResult {
        clean,
        best_effort,
        quarantined,
        total_rows,
        total_tco2e,
        processing_errors: Vec::new(),
    }
}

pub struct Dispatcher;

impl Dispatcher {
    pub fn process_file(filename: &str, data: &[u8]) -> DispatchResult {
        let file_type = detect_file_type(filename);
        
        let rows_res = match file_type {
            FileType::Csv => parse_csv(data),
            FileType::Xlsx => parse_xlsx(data),
            FileType::Json => parse_json(data),
            FileType::Xml => Err("XML parsing not implemented yet".to_string()),
            FileType::Unknown => Err(format!("Unknown file type for: {}", filename)),
        };

        match rows_res {
            Ok(rows) => process_rows(rows),
            Err(e) => DispatchResult {
                clean: Vec::new(),
                best_effort: Vec::new(),
                quarantined: Vec::new(),
                total_rows: 0,
                total_tco2e: 0.0,
                processing_errors: vec![e],
            },
        }
    }
}
