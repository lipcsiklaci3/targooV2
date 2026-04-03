use libsql::{Connection, params};
use sha2::{Sha256, Digest};
use hex;

pub struct PhysicsResult {
    pub canonical_value: f64,
    pub canonical_unit: String,
    pub tco2e: f64,
    pub factor_source: String,
    pub row_sha256: String,
}

pub enum PhysicsError {
    NonNumeric,
    PhysicalDomainMismatch { unit: String, category: String, expected: String },
    NoConversionPath { from: String, to: String },
    NoEmissionFactor(String),
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::NonNumeric => write!(f, "Value is not a valid number"),
            PhysicsError::PhysicalDomainMismatch { unit, category, expected } => {
                write!(f, "Physical Domain Mismatch: Unit '{}' is not allowed for category '{}'. Expected one of: {}", unit, category, expected)
            }
            PhysicsError::NoConversionPath { from, to } => write!(f, "No conversion path from {} to {}", from, to),
            PhysicsError::NoEmissionFactor(target) => write!(f, "No emission factor found for target: {}", target),
        }
    }
}

pub async fn get_physical_domain(conn: &Connection, unit: &str) -> String {
    // 1. Check matrix for domain
    let mut rows = conn.query(
        "SELECT physical_domain FROM unit_conversion_matrix WHERE from_unit = ? LIMIT 1",
        params![unit]
    ).await.unwrap();

    if let Ok(Some(row)) = rows.next().await {
        return row.get::<String>(0).unwrap();
    }

    // 2. Fallback for canonical units not in the "from" column
    match unit.to_lowercase().as_str() {
        "kwh" | "mwh" | "gj" | "mmbtu" => "ENERGY".to_string(),
        "liters" | "m3" | "gallons_us" | "gallons_uk" => "VOLUME".to_string(),
        "kg" | "tonne" | "metric_tons" | "short_tons" | "lbs" => "MASS".to_string(),
        "km" | "miles" => "DISTANCE".to_string(),
        "usd" | "gbp" | "currency" => "CURRENCY".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

pub fn get_allowed_domains(category: &str) -> Vec<&str> {
    let cat = category.to_lowercase();
    if cat.contains("stationarycombustion") {
        vec!["ENERGY", "VOLUME"]
    } else if cat.contains("mobilecombustion") {
        vec!["VOLUME", "DISTANCE"]
    } else if cat.contains("electricity") {
        vec!["ENERGY"]
    } else if cat.contains("refrigerant") || cat.contains("fugitive") {
        vec!["MASS"]
    } else if cat.contains("purchasedgoods") && cat.contains("spend") {
        vec!["CURRENCY"]
    } else if cat.contains("wastegenerated") {
        vec!["MASS", "VOLUME"]
    } else {
        vec!["ENERGY", "VOLUME", "MASS", "DISTANCE", "CURRENCY"]
    }
}

pub async fn calculate_refined_emissions(
    conn: &Connection,
    job_id: &str,
    row_number: i64,
    raw_value: &str,
    input_unit: &str,
    target_category: &str,
) -> Result<PhysicsResult, PhysicsError> {
    // 1. Layer 1: Numeric Validation
    let val: f64 = raw_value.parse().map_err(|_| PhysicsError::NonNumeric)?;

    // 2. Layer 2: Physical Domain Validation (SAD 4.2)
    let unit_domain = get_physical_domain(conn, input_unit).await;
    let allowed_domains = get_allowed_domains(target_category);

    if !allowed_domains.contains(&unit_domain.as_str()) {
        return Err(PhysicsError::PhysicalDomainMismatch {
            unit: input_unit.to_string(),
            category: target_category.to_string(),
            expected: allowed_domains.join(", "),
        });
    }

    // 3. Lookup Emission Factor
    let mut factor_rows = conn.query(
        "SELECT factor_value, base_unit, source FROM emission_factors WHERE esrs_target = ?",
        params![target_category]
    ).await.unwrap();

    let (ef_value, ef_unit, ef_source) = if let Ok(Some(row)) = factor_rows.next().await {
        (
            row.get::<f64>(0).unwrap(),
            row.get::<String>(1).unwrap(),
            row.get::<String>(2).unwrap(),
        )
    } else {
        return Err(PhysicsError::NoEmissionFactor(target_category.to_string()));
    };

    // 4. Layer 3: Canonical Conversion
    let mut multiplier = 1.0;
    if input_unit != ef_unit {
        let mut conv_rows = conn.query(
            "SELECT multiplier FROM unit_conversion_matrix WHERE from_unit = ? AND to_unit = ?",
            params![input_unit, ef_unit.clone()] // Fixed: Clone to prevent move
        ).await.unwrap();

        if let Ok(Some(row)) = conv_rows.next().await {
            multiplier = row.get::<f64>(0).unwrap();
        } else {
            // Identity check if they are functionally the same
            if input_unit.to_lowercase() != ef_unit.to_lowercase() {
                return Err(PhysicsError::NoConversionPath {
                    from: input_unit.to_string(),
                    to: ef_unit,
                });
            }
        }
    }

    let canonical_value = val * multiplier;

    // 5. Layer 5: tCO2e Calculation
    let tco2e = (canonical_value * ef_value) / 1000.0;

    // 6. Layer 6: Cryptographic Hash (SAD 5.3)
    let hash_input = format!(
        "{}{}{}{}{:.6}",
        job_id, row_number, raw_value, target_category, tco2e
    );
    let row_sha256 = hex::encode(Sha256::digest(hash_input.as_bytes()));

    Ok(PhysicsResult {
        canonical_value,
        canonical_unit: ef_unit,
        tco2e,
        factor_source: ef_source,
        row_sha256,
    })
}
