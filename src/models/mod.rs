// Domain models and data structures
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawFile {
    pub name: String,
    pub content_type: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingResult {
    pub esrs_target: String,
    pub raw_header: String,
    pub raw_value: String,
    pub canonical_value: f64,
    pub canonical_unit: String,
    pub tco2e: f64,
    pub emission_factor: f64,
    pub factor_source: String,
    pub confidence: f64,
    pub status: String, // "clean", "best_effort", "quarantined"
    pub warning: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: Uuid,
    pub reason: String,
    pub original_data: String,
}
