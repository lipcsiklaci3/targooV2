use axum::{
    extract::{Path, Multipart},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;
use tower_http::cors::{Any, CorsLayer};

use crate::pipeline::dispatcher::Dispatcher;

#[derive(Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub status: String,
    pub total_files: usize,
    pub total_rows: usize,
    pub total_tco2e: f64,
    pub clean_rows: usize,
    pub best_effort_rows: usize,
    pub quarantined_rows: usize,
    pub processing_errors: Vec<String>,
}

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "service": "Targoo V2 ESG Data Refinery"
    }))
}

pub async fn upload_job(mut multipart: Multipart) -> Result<Json<JobSummary>, (StatusCode, String)> {
    let mut total_files = 0;
    let mut total_rows = 0;
    let mut total_tco2e = 0.0;
    let mut clean_rows = 0;
    let mut best_effort_rows = 0;
    let mut quarantined_rows = 0;
    let mut processing_errors = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        total_files += 1;
        let res = Dispatcher::process_file(&filename, &data);
        
        total_rows += res.total_rows;
        total_tco2e += res.total_tco2e;
        clean_rows += res.clean.len();
        best_effort_rows += res.best_effort.len();
        quarantined_rows += res.quarantined.len();
        processing_errors.extend(res.processing_errors);
    }

    let summary = JobSummary {
        job_id: Uuid::new_v4().to_string(),
        status: "complete".to_string(),
        total_files,
        total_rows,
        total_tco2e,
        clean_rows,
        best_effort_rows,
        quarantined_rows,
        processing_errors,
    };

    Ok(Json(summary))
}

pub async fn get_job_status(Path(job_id): Path<String>) -> Json<Value> {
    Json(json!({
        "job_id": job_id,
        "status": "complete",
        "progress": 100,
        "message": "Job processed successfully (mock response)"
    }))
}

pub fn create_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/jobs/upload", post(upload_job))
        .route("/jobs/:job_id/status", get(get_job_status))
        .layer(cors)
}
