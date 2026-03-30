// Entry point for the Shuttle-powered Axum web server
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/health", get(health_check));

    Ok(router.into())
}
