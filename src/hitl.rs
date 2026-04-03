use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use libsql::{Connection, params};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct HitlResolution {
    pub hitl_id: i32,
    pub target_category: String,
    pub industry: String,
    pub expected_unit: String,
    pub normalized_unit: String,
    pub jurisdiction: String,
}

pub async fn resolve_hitl_endpoint(
    State(conn): State<Arc<Connection>>,
    Json(payload): Json<HitlResolution>,
) -> Json<Value> {
    // 1. Fetch the raw header from hitl_queue
    let mut rows = conn.query("SELECT raw_header, job_id FROM hitl_queue WHERE id = ?", params![payload.hitl_id]).await.unwrap();
    if let Ok(Some(row)) = rows.next().await {
        let raw_header: String = row.get(0).unwrap();
        let job_id: String = row.get(1).unwrap();

        let resolved_mapping = json!({
            "target_category": payload.target_category,
            "industry": payload.industry,
            "expected_unit": payload.expected_unit,
            "normalized_unit": payload.normalized_unit,
            "jurisdiction": payload.jurisdiction
        }).to_string();

        // 2. Update HITL Queue
        let _ = conn.execute(
            "UPDATE hitl_queue SET status = 'RESOLVED', resolved_mapping = ? WHERE id = ?",
            params![resolved_mapping, payload.hitl_id]
        ).await;

        // 3. Auto-Learning: Update Mapping Dictionary
        let kw_json = serde_json::to_string(&vec![raw_header.clone()]).unwrap();
        let _ = conn.execute(
            "INSERT INTO mapping_dictionary (jurisdiction, target_category, industry, expected_unit, normalized_unit, keywords) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.jurisdiction.clone(),
                payload.target_category.clone(),
                payload.industry.clone(),
                payload.expected_unit.clone(),
                payload.normalized_unit.clone(),
                kw_json
            ]
        ).await;

        // 4. Update Job Status to allow processing to continue
        let _ = conn.execute(
            "UPDATE job_registry SET status = 'PROCESSING' WHERE job_id = ?",
            params![job_id.clone()]
        ).await;

        println!("[HITL] Resolved header '{}' -> Category '{}'. Job '{}' resumed.", raw_header, payload.target_category, job_id);

        Json(json!({"status": "RESOLVED", "header": raw_header}))
    } else {
        Json(json!({"status": "ERROR", "message": "HITL item not found"}))
    }
}
