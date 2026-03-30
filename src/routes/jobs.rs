use axum::{
    extract::{Path, Multipart},
    http::{header, StatusCode, Response},
    routing::{get, post},
    body::Body,
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::pipeline::dispatcher::{Dispatcher, DispatchResult};
use crate::output::FritzPackage;

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

pub async fn upload_job(mut multipart: Multipart) -> Result<Response<Body>, (StatusCode, String)> {
    let mut combined = DispatchResult {
        clean: Vec::new(),
        best_effort: Vec::new(),
        quarantined: Vec::new(),
        total_rows: 0,
        total_tco2e: 0.0,
        processing_errors: Vec::new(),
    };
    let mut total_files = 0;

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        total_files += 1;
        let res = Dispatcher::process_file(&filename, &data);
        
        combined.clean.extend(res.clean);
        combined.best_effort.extend(res.best_effort);
        combined.quarantined.extend(res.quarantined);
        combined.total_rows += res.total_rows;
        combined.total_tco2e += res.total_tco2e;
        combined.processing_errors.extend(res.processing_errors);
    }

    if total_files == 0 {
        return Err((StatusCode::BAD_REQUEST, "No files uploaded".to_string()));
    }

    match FritzPackage::assemble(&combined) {
        Ok(zip_bytes) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/zip")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"Fritz_Package.zip\"")
                .body(Body::from(zip_bytes))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(response)
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to assemble ZIP: {}", e))),
    }
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
