// Entry point for the Shuttle-powered Axum web server
use shuttle_axum::ShuttleAxum;
use std::sync::Arc;
use crate::db::Database;
use crate::pipeline::groq::GroqClient;

pub mod db;
pub mod models;
pub mod pipeline;
pub mod routes;
pub mod output;

pub struct AppState {
    pub db: Option<Database>,
    pub groq: Option<GroqClient>,
}

#[shuttle_runtime::main]
async fn main() -> ShuttleAxum {
    let database_url = std::env::var("TURSO_DATABASE_URL");
    let auth_token = std::env::var("TURSO_AUTH_TOKEN");
    let groq_api_key = std::env::var("GROQ_API_KEY");

    let db = match (database_url, auth_token) {
        (Ok(url), Ok(token)) => {
            match Database::new(&url, &token).await {
                Ok(database) => {
                    if let Err(e) = database.initialize().await {
                        eprintln!("Failed to initialize database: {}", e);
                        None
                    } else {
                        println!("Connected to Turso database");
                        Some(database)
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to Turso: {}", e);
                    None
                }
            }
        }
        _ => {
            println!("Running without database - results will not be persisted");
            None
        }
    };

    let groq = match groq_api_key {
        Ok(key) => {
            println!("Groq AI client initialized");
            Some(GroqClient::new(key))
        },
        Err(_) => {
            println!("Groq API key not found - AI fallback disabled");
            None
        }
    };

    let state = Arc::new(AppState { db, groq });
    let router = routes::jobs::create_router(state);

    Ok(router.into())
}
