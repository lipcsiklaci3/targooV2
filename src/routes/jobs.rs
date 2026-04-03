use axum::{
    extract::{Path, Multipart, State},
    http::{header, StatusCode, Response},
    routing::{get, post},
    body::Body,
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use std::sync::Arc;
use uuid::Uuid;

use crate::pipeline::dispatcher::{Dispatcher, DispatchResult};
use crate::output::FritzPackage;
use crate::AppState;

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "service": "Targoo V2 ESG Data Refinery"
    }))
}

pub async fn upload_job(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart
) -> Result<Response<Body>, (StatusCode, String)> {
    let mut combined = DispatchResult {
        clean: Vec::new(),
        best_effort: Vec::new(),
        quarantined: Vec::new(),
        total_rows: 0,
        total_tco2e: 0.0,
        processing_errors: Vec::new(),
    };
    let mut total_files = 0;
    let job_id = Uuid::new_v4().to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        total_files += 1;
        let res = Dispatcher::process_file(&filename, &data).await;
        
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

    // Persist to database if available
    if let Some(db) = &state.db {
        db.save_job(&job_id, total_files, &combined).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        db.save_ledger_entries(&job_id, &combined).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    match FritzPackage::assemble(&combined) {
        Ok(zip_bytes) => {
            let summary_json = json!({
                "job_id": job_id,
                "total_rows": combined.total_rows,
                "total_tco2e": combined.total_tco2e,
                "clean_rows": combined.clean.len(),
                "best_effort_rows": combined.best_effort.len(),
                "quarantined_rows": combined.quarantined.len(),
            }).to_string();

            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/zip")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"Fritz_Package_{}.zip\"", job_id))
                .header("X-Job-Summary", summary_json)
                .header("Access-Control-Expose-Headers", "X-Job-Summary")
                .body(Body::from(zip_bytes))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(response)
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to assemble ZIP: {}", e))),
    }
}

pub async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>
) -> Result<Json<Value>, (StatusCode, String)> {
    if let Some(db) = &state.db {
        match db.get_job(&job_id).await {
            Ok(Some(job)) => Ok(Json(job)),
            Ok(None) => Err((StatusCode::NOT_FOUND, "Job not found".to_string())),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
        }
    } else {
        Ok(Json(json!({
            "job_id": job_id,
            "status": "complete",
            "message": "Running in stateless mode (no DB)"
        })))
    }
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/jobs/upload", post(upload_job))
        .route("/jobs/:job_id/status", get(get_job_status))
        .with_state(state)
        .layer(cors)
}
