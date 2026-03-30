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

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingResult {
    pub source_field: String,
    pub target_field: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: Uuid,
    pub reason: String,
    pub original_data: String,
}
