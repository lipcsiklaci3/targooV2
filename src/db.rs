use libsql::{Builder, Connection, params};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize, Serialize)]
struct DictEntry {
    pub jurisdiction: String,
    pub target_category: String,
    pub industry: String,
    pub expected_unit: String,
    pub normalized_unit: String,
    pub keywords: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct EmissionFactorEntry {
    #[serde(rename = "ESRS_Target")]
    pub esrs_target: String,
    #[serde(rename = "Base_Unit")]
    pub base_unit: String,
    #[serde(rename = "Emission_Factor_kgCO2e")]
    pub factor_value: f64,
    #[serde(rename = "Source_Authority")]
    pub source: String,
}

pub async fn init_db() -> Connection {
    let db = Builder::new_local("targoo.db").build().await.unwrap();
    let conn = db.connect().unwrap();

    // 1. Mapping Dictionary
    conn.execute("CREATE TABLE IF NOT EXISTS mapping_dictionary (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        jurisdiction TEXT NOT NULL,
        target_category TEXT NOT NULL,
        industry TEXT NOT NULL,
        expected_unit TEXT NOT NULL,
        normalized_unit TEXT NOT NULL,
        keywords TEXT NOT NULL
    )", ()).await.unwrap();

    // 2. ESG Ledger (WORM Audit Trail)
    conn.execute("CREATE TABLE IF NOT EXISTS esg_ledger (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id TEXT NOT NULL,
        source_filename TEXT NOT NULL,
        source_row_number INTEGER NOT NULL,
        source_header_raw TEXT NOT NULL,
        raw_value TEXT NOT NULL,
        canonical_value REAL,
        canonical_unit TEXT,
        ef_factor_code TEXT,
        tco2e REAL,
        row_sha256 TEXT NOT NULL UNIQUE,
        status TEXT NOT NULL,
        created_at INTEGER NOT NULL DEFAULT (unixepoch())
    )", ()).await.unwrap();

    // WORM PROTECTIONS
    let _ = conn.execute("CREATE TRIGGER IF NOT EXISTS prevent_ledger_update BEFORE UPDATE ON esg_ledger 
        BEGIN SELECT RAISE(ABORT, 'ESG_LEDGER_WORM: Modification not permitted'); END;", ()).await;
    let _ = conn.execute("CREATE TRIGGER IF NOT EXISTS prevent_ledger_delete BEFORE DELETE ON esg_ledger 
        BEGIN SELECT RAISE(ABORT, 'ESG_LEDGER_WORM: Deletion not permitted'); END;", ()).await;

    // 3. Emission Factors
    conn.execute("CREATE TABLE IF NOT EXISTS emission_factors (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        esrs_target TEXT NOT NULL UNIQUE,
        base_unit TEXT NOT NULL,
        factor_value REAL NOT NULL,
        source TEXT NOT NULL
    )", ()).await.unwrap();

    // 4. Unit Conversion Matrix
    conn.execute("CREATE TABLE IF NOT EXISTS unit_conversion_matrix (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        from_unit TEXT NOT NULL,
        to_unit TEXT NOT NULL,
        multiplier REAL NOT NULL,
        physical_domain TEXT NOT NULL,
        UNIQUE (from_unit, to_unit)
    )", ()).await.unwrap();

    // 5. XBRL Tag Registry
    conn.execute("CREATE TABLE IF NOT EXISTS xbrl_tag_registry (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        tag_code TEXT NOT NULL UNIQUE,
        tag_name_us TEXT,
        tag_name_uk TEXT,
        ghg_scope TEXT NOT NULL
    )", ()).await.unwrap();

    // 6. Job Registry
    conn.execute("CREATE TABLE IF NOT EXISTS job_registry (
        job_id TEXT PRIMARY KEY,
        status TEXT NOT NULL,
        total_rows INTEGER NOT NULL DEFAULT 0,
        processed_rows INTEGER NOT NULL DEFAULT 0
    )", ()).await.unwrap();

    // 7. HITL Queue
    conn.execute("CREATE TABLE IF NOT EXISTS hitl_queue (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id TEXT NOT NULL,
        raw_header TEXT NOT NULL,
        sample_values TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'AWAITING_HUMAN'
    )", ()).await.unwrap();

    conn
}

pub async fn seed_infrastructure(conn: &Connection) {
    let mut rows = conn.query("SELECT COUNT(*) FROM unit_conversion_matrix", ()).await.unwrap();
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    if count == 0 {
        let conversions = vec![
            ("therms", "kWh", 29.3, "ENERGY"),
            ("gallons_us", "liters", 3.785, "VOLUME"),
            ("gallons_uk", "liters", 4.546, "VOLUME"),
            ("miles", "km", 1.609, "DISTANCE"),
            ("lbs", "kg", 0.4536, "MASS"),
            ("short_tons", "tonne", 0.9072, "MASS"),
        ];
        for (from, to, mult, dom) in conversions {
            let _ = conn.execute(
                "INSERT INTO unit_conversion_matrix (from_unit, to_unit, multiplier, physical_domain) VALUES (?, ?, ?, ?)",
                params![from, to, mult, dom]
            ).await;
        }
        println!("[SYSTEM] Unit Conversion Matrix seeded.");
    }

    let mut rows = conn.query("SELECT COUNT(*) FROM xbrl_tag_registry", ()).await.unwrap();
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    if count == 0 {
        let tags = vec![
            ("SEC_GHG_SCOPE1", "dei:Scope1GHGEmissions", "esrs:GrossScope1GHGEmissions", "SCOPE_1"),
            ("SEC_GHG_SCOPE2_LOC", "dei:Scope2GHGEmissionsLocationBased", "esrs:Scope2LocationBasedGHGEmissions", "SCOPE_2"),
            ("SEC_GHG_SCOPE3_CAT1", "dei:Scope3Cat1PurchasedGoods", "esrs:Scope3Category1PurchGoods", "SCOPE_3"),
        ];
        for (code, us, uk, scope) in tags {
            let _ = conn.execute(
                "INSERT INTO xbrl_tag_registry (tag_code, tag_name_us, tag_name_uk, ghg_scope) VALUES (?, ?, ?, ?)",
                params![code, us, uk, scope]
            ).await;
        }
        println!("[SYSTEM] XBRL Tag Registry seeded.");
    }
}

pub async fn seed_all(conn: &Connection) {
    seed_infrastructure(conn).await;
    let mut rows = conn.query("SELECT COUNT(*) FROM mapping_dictionary", ()).await.unwrap();
    let dict_count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    if dict_count == 0 {
        if let Ok(content) = fs::read_to_string("data/dictionary.json") {
            if let Ok(entries) = serde_json::from_str::<Vec<DictEntry>>(&content) {
                for entry in &entries {
                    let kw_json = serde_json::to_string(&entry.keywords).unwrap();
                    let _ = conn.execute(
                        "INSERT INTO mapping_dictionary (jurisdiction, target_category, industry, expected_unit, normalized_unit, keywords) VALUES (?, ?, ?, ?, ?, ?)",
                        params![entry.jurisdiction.clone(), entry.target_category.clone(), entry.industry.clone(), entry.expected_unit.clone(), entry.normalized_unit.clone(), kw_json]
                    ).await;
                }
                println!("[SYSTEM] US/UK Dictionary seeded successfully.");
            }
        }
    }
    let mut rows = conn.query("SELECT COUNT(*) FROM emission_factors", ()).await.unwrap();
    let factor_count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    if factor_count == 0 {
        if let Ok(content) = fs::read_to_string("data/emission_factors.json") {
            if let Ok(entries) = serde_json::from_str::<Vec<EmissionFactorEntry>>(&content) {
                for entry in &entries {
                    let _ = conn.execute(
                        "INSERT INTO emission_factors (esrs_target, base_unit, factor_value, source) VALUES (?, ?, ?, ?)",
                        params![entry.esrs_target.clone(), entry.base_unit.clone(), entry.factor_value, entry.source.clone()]
                    ).await;
                }
                println!("[SYSTEM] Emission Factors seeded successfully.");
            }
        }
    }
}
