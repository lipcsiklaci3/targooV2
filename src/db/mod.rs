use libsql::{Builder, Connection, named_params};
use crate::pipeline::dispatcher::DispatchResult;
pub mod schema;
use schema::*;
use chrono::Utc;
use serde_json::{json, Value};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(url: &str, token: &str) -> Result<Self, String> {
        let db = Builder::new_remote(url.to_string(), token.to_string())
            .build()
            .await
            .map_err(|e| e.to_string())?;
        
        let conn = db.connect().map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    pub async fn initialize(&self) -> Result<(), String> {
        self.conn.execute(CREATE_JOBS_TABLE, ()).await.map_err(|e| e.to_string())?;
        self.conn.execute(CREATE_ESG_LEDGER_TABLE, ()).await.map_err(|e| e.to_string())?;
        self.conn.execute(CREATE_QUARANTINE_TABLE, ()).await.map_err(|e| e.to_string())?;
        self.conn.execute(CREATE_MAPPING_DICTIONARY_TABLE, ()).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn save_job(&self, job_id: &str, total_files: usize, result: &DispatchResult) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO jobs (id, status, total_files, total_rows, total_tco2e, clean_rows, best_effort_rows, quarantined_rows, created_at, completed_at) 
             VALUES (:id, :status, :total_files, :total_rows, :total_tco2e, :clean_rows, :best_effort_rows, :quarantined_rows, :created_at, :completed_at)",
            named_params! {
                ":id": job_id.to_string(),
                ":status": "complete".to_string(),
                ":total_files": total_files as i64,
                ":total_rows": result.total_rows as i64,
                ":total_tco2e": result.total_tco2e,
                ":clean_rows": result.clean.len() as i64,
                ":best_effort_rows": result.best_effort.len() as i64,
                ":quarantined_rows": result.quarantined.len() as i64,
                ":created_at": now.clone(),
                ":completed_at": now,
            },
        ).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn save_ledger_entries(&self, job_id: &str, result: &DispatchResult) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();

        // Combined clean + best_effort for ledger
        let ledger_items: Vec<_> = result.clean.iter().chain(result.best_effort.iter()).collect();
        for chunk in ledger_items.chunks(100) {
            for item in chunk {
                self.conn.execute(
                    "INSERT INTO esg_ledger (job_id, esrs_target, raw_header, raw_value, canonical_value, canonical_unit, tco2e, emission_factor, factor_source, confidence, status, warning, error, processed_at)
                     VALUES (:job_id, :esrs_target, :raw_header, :raw_value, :canonical_value, :canonical_unit, :tco2e, :emission_factor, :factor_source, :confidence, :status, :warning, :error, :processed_at)",
                    named_params! {
                        ":job_id": job_id.to_string(),
                        ":esrs_target": item.esrs_target.clone(),
                        ":raw_header": item.raw_header.clone(),
                        ":raw_value": item.raw_value.clone(),
                        ":canonical_value": item.canonical_value,
                        ":canonical_unit": item.canonical_unit.clone(),
                        ":tco2e": item.tco2e,
                        ":emission_factor": item.emission_factor,
                        ":factor_source": item.factor_source.clone(),
                        ":confidence": item.confidence,
                        ":status": item.status.clone(),
                        ":warning": item.warning.clone().unwrap_or_default(),
                        ":error": item.error.clone().unwrap_or_default(),
                        ":processed_at": now.clone(),
                    }
                ).await.map_err(|e| e.to_string())?;
            }
        }

        // Quarantined items
        for chunk in result.quarantined.chunks(100) {
            for item in chunk {
                self.conn.execute(
                    "INSERT INTO quarantine_log (job_id, esrs_target, raw_header, raw_value, error, confidence, created_at)
                     VALUES (:job_id, :esrs_target, :raw_header, :raw_value, :error, :confidence, :created_at)",
                    named_params! {
                        ":job_id": job_id.to_string(),
                        ":esrs_target": item.esrs_target.clone(),
                        ":raw_header": item.raw_header.clone(),
                        ":raw_value": item.raw_value.clone(),
                        ":error": item.error.clone().unwrap_or_default(),
                        ":confidence": item.confidence,
                        ":created_at": now.clone(),
                    }
                ).await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Option<Value>, String> {
        let mut rows = self.conn.query(
            "SELECT * FROM jobs WHERE id = :id",
            named_params! { ":id": job_id.to_string() }
        ).await.map_err(|e| e.to_string())?;
        
        if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            Ok(Some(json!({
                "id": row.get::<String>(0).unwrap_or_default(),
                "status": row.get::<String>(1).unwrap_or_default(),
                "total_files": row.get::<i64>(2).unwrap_or_default(),
                "total_rows": row.get::<i64>(3).unwrap_or_default(),
                "total_tco2e": row.get::<f64>(4).unwrap_or_default(),
                "clean_rows": row.get::<i64>(5).unwrap_or_default(),
                "best_effort_rows": row.get::<i64>(6).unwrap_or_default(),
                "quarantined_rows": row.get::<i64>(7).unwrap_or_default(),
                "output_zip_url": row.get::<Option<String>>(8).unwrap_or_default(),
                "created_at": row.get::<String>(9).unwrap_or_default(),
                "completed_at": row.get::<Option<String>>(10).unwrap_or_default(),
            })))
        } else {
            Ok(None)
        }
    }
}
