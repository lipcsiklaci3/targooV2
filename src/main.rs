// Entry point for the Shuttle-powered Axum web server
use shuttle_axum::ShuttleAxum;

pub mod db;
pub mod models;
pub mod pipeline;
pub mod routes;

#[shuttle_runtime::main]
async fn main() -> ShuttleAxum {
    let router = routes::jobs::create_router();

    Ok(router.into())
}
