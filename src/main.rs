mod db;
mod ingest;
mod physics;
mod exporter;
mod hitl;
mod processor;

use axum::{routing::{get, post}, Router, Json, extract::{Path, State}};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;
use std::sync::Arc;
use libsql::{Connection, params};

#[tokio::main]
async fn main() {
    // 1. Initialize Database
    let conn = db::init_db().await;
    let shared_conn = Arc::new(conn);
    println!("[SYSTEM] Local libSQL database initialized: targoo.db");

    // 2. SEEDER: Load infrastructure, dictionary, and emission factors
    db::seed_all(&shared_conn).await;

    // 3. CORS: Truly permissive for GitHub Codespaces
    let cors = CorsLayer::permissive()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/upload", post(ingest::upload_handler))
        .route("/api/download/:job_id", get(exporter::download_package))
        .route("/api/jobs/:job_id/status", get(get_job_status))
        .route("/api/jobs/:job_id/hitl", get(get_hitl_items))
        .route("/api/hitl/resolve", post(hitl::resolve_hitl_endpoint))
        .layer(cors)
        .with_state(shared_conn);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("[LOG] Targoo V2 Kernel is running on port 3000...");
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "SYSTEM_ONLINE",
        "message": "Targoo V2 Engine Ready and Seeded"
    }))
}

async fn get_job_status(
    Path(job_id): Path<String>,
    State(conn): State<Arc<Connection>>,
) -> Json<Value> {
    let mut rows = conn.query("SELECT status, total_rows, processed_rows FROM job_registry WHERE job_id = ?", params![job_id.clone()]).await.unwrap();
    if let Ok(Some(row)) = rows.next().await {
        Json(json!({
            "job_id": job_id,
            "status": row.get::<String>(0).unwrap(),
            "total_rows": row.get::<i64>(1).unwrap(),
            "processed_rows": row.get::<i64>(2).unwrap()
        }))
    } else {
        Json(json!({"status": "NOT_FOUND"}))
    }
}

async fn get_hitl_items(
    Path(job_id): Path<String>,
    State(conn): State<Arc<Connection>>,
) -> Json<Value> {
    let mut rows = conn.query("SELECT id, raw_header, sample_values FROM hitl_queue WHERE job_id = ? AND status = 'AWAITING_HUMAN'", params![job_id.clone()]).await.unwrap();
    let mut items = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        items.push(json!({
            "hitl_id": row.get::<i64>(0).unwrap(),
            "raw_header": row.get::<String>(1).unwrap(),
            "sample_values": row.get::<String>(2).unwrap()
        }));
    }
    Json(json!({ "job_id": job_id, "hitl_items": items }))
}
