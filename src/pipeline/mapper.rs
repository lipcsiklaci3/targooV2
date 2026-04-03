// ESG Data Mapping Logic
use crate::models::MappingResult;
use crate::pipeline::validator::{UnitValidator, RangeGuard, RangeCheckResult};

pub struct Mapper;

struct DictionaryEntry {
    esrs_target: String,
    canonical_unit: String,
    conversion_factor: f64,
}

impl Mapper {
    fn normalize(text: &str) -> String {
        text.to_lowercase()
            .replace('_', " ")
            .replace('-', " ")
            .trim()
            .to_string()
    }

    fn lookup_dictionary(normalized_header: &str) -> Option<DictionaryEntry> {
        let triggers = vec![
            // Format: (trigger_keywords, esrs_target, canonical_unit, conversion_factor)
            (vec!["stromverbrauch", "strom", "electricity", "power consumption"], "E1-6_Scope2_Electricity", "kWh", 1.0),
            (vec!["erdgasverbrauch", "gasverbrauch", "erdgas", "natural gas"], "E1-1_Scope1_NaturalGas", "kWh", 1.0),
            (vec!["diesel"], "E1-1_Scope1_Diesel", "liter", 1.0),
            (vec!["benzin", "petrol"], "E1-1_Scope1_Petrol", "liter", 1.0),
            (vec!["heizöl", "heizoel", "heating oil"], "E1-1_Scope1_Heating_Oil", "liter", 1.0),
            (vec!["straßentransport", "strassentransport", "road freight", "lkw"], "E1-3_Scope3_Road_Freight", "tkm", 1.0),
            (vec!["luftfracht", "air freight"], "E1-3_Scope3_Air_Freight", "tkm", 1.0),
            (vec!["schienentransport", "rail freight", "bahn"], "E1-3_Scope3_Rail_Freight", "tkm", 1.0),
            (vec!["kältemittel", "kaeltemittel", "refrigerant"], "E1-2_Scope1_Refrigerants", "kg", 1.0),
            (vec!["abfall deponie", "abfall", "waste", "deponie"], "E1-3_Scope3_Waste_Landfill", "kg", 1.0),
        ];

        // Flatten all keywords and sort by length descending to prioritize specific matches
        let mut flattened = Vec::new();
        for (keywords, target, unit, factor) in triggers {
            for kw in keywords {
                flattened.push((kw, target, unit, factor));
            }
        }
        flattened.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (kw, target, unit, factor) in flattened {
            if normalized_header.contains(kw) {
                return Some(DictionaryEntry {
                    esrs_target: target.to_string(),
                    canonical_unit: unit.to_string(),
                    conversion_factor: factor,
                });
            }
        }
        
        None
    }

    fn get_emission_factor(esrs_target: &str) -> (f64, String) {
        match esrs_target {
            "E1-6_Scope2_Electricity" => (0.354, "IEA 2023 Germany".to_string()),
            "E1-1_Scope1_NaturalGas" => (0.202, "DEFRA 2024".to_string()),
            "E1-1_Scope1_Diesel" => (2.68, "DEFRA 2024".to_string()),
            "E1-1_Scope1_Petrol" => (2.31, "DEFRA 2024".to_string()),
            "E1-1_Scope1_Heating_Oil" => (2.67, "DEFRA 2024".to_string()),
            "E1-3_Scope3_Road_Freight" => (0.15, "DEFRA 2024 HGV Avg".to_string()),
            "E1-3_Scope3_Air_Freight" => (1.52, "DEFRA 2024 Cargo Plane".to_string()),
            "E1-3_Scope3_Rail_Freight" => (0.02, "DEFRA 2024 Freight Train".to_string()),
            "E1-2_Scope1_Refrigerants" => (1430.0, "GWP R134a".to_string()),
            "E1-3_Scope3_Waste_Landfill" => (0.58, "DEFRA 2024 Mixed Waste".to_string()),
            _ => (0.0, "Unknown".to_string()),
        }
    }

    pub async fn process(
        raw_header: &str,
        raw_value: &str,
        input_unit_raw: &str,
    ) -> MappingResult {
        if input_unit_raw.is_empty() {
            // If no unit hint, skip unit family check entirely and proceed with factor's canonical unit
            // This is valid: many headers don't have unit hints
        }

        let normalized_header = Self::normalize(raw_header);

        // Step 1: Check Dictionary with CONTAINS matching
        let entry = match Self::lookup_dictionary(&normalized_header) {
            Some(entry) => entry,
            None => {
                return MappingResult {
                    esrs_target: "UNKNOWN".to_string(),
                    raw_header: raw_header.to_string(),
                    raw_value: raw_value.to_string(),
                    canonical_value: 0.0,
                    canonical_unit: "UNKNOWN".to_string(),
                    tco2e: 0.0,
                    emission_factor: 0.0,
                    factor_source: "None".to_string(),
                    confidence: 0.0,
                    status: "quarantined".to_string(),
                    warning: None,
                    error: Some(format!(
                        "Unknown header: {} | Unit hint: {} | Please map manually",
                        raw_header, input_unit_raw
                    )),
                };
            }
        };

        let target = entry.esrs_target;
        let factor_unit_str = entry.canonical_unit;
        let dictionary_factor = entry.conversion_factor;

        // Step 2: Parse raw value
        let val: f64 = raw_value.parse().unwrap_or(0.0);
        let mut canonical_value = val * dictionary_factor;
        let mut final_unit_str = factor_unit_str.clone();

        // Step 3: Unit Validation & Normalization
        if !input_unit_raw.is_empty() {
            match UnitValidator::parse(input_unit_raw) {
                Ok(input_unit) => {
                    // Step 4: Family check
                    match UnitValidator::parse(&factor_unit_str) {
                        Ok(factor_unit) => {
                            if let Err(e) = UnitValidator::check_family_match(&input_unit, &factor_unit) {
                                return MappingResult {
                                    esrs_target: target,
                                    raw_header: raw_header.to_string(),
                                    raw_value: raw_value.to_string(),
                                    canonical_value: 0.0,
                                    canonical_unit: factor_unit_str,
                                    tco2e: 0.0,
                                    emission_factor: 0.0,
                                    factor_source: "None".to_string(),
                                    confidence: 0.0,
                                    status: "quarantined".to_string(),
                                    warning: None,
                                    error: Some(format!("STEP_4_FAIL: {}", e)),
                                };
                            }
                            // Step 5: Conversion to base unit
                            canonical_value = UnitValidator::convert_to_base(val, &input_unit) * dictionary_factor;
                            final_unit_str = factor_unit.to_string();
                        }
                        Err(e) => {
                            return MappingResult {
                                esrs_target: target,
                                raw_header: raw_header.to_string(),
                                raw_value: raw_value.to_string(),
                                canonical_value: 0.0,
                                canonical_unit: factor_unit_str,
                                tco2e: 0.0,
                                emission_factor: 0.0,
                                factor_source: "None".to_string(),
                                confidence: 0.0,
                                status: "quarantined".to_string(),
                                warning: None,
                                error: Some(format!("STEP_4_FAIL: Dictionary unit error: {}", e)),
                            };
                        }
                    }
                }
                Err(e) => {
                    return MappingResult {
                        esrs_target: target,
                        raw_header: raw_header.to_string(),
                        raw_value: raw_value.to_string(),
                        canonical_value: 0.0,
                        canonical_unit: factor_unit_str,
                        tco2e: 0.0,
                        emission_factor: 0.0,
                        factor_source: "None".to_string(),
                        confidence: 0.0,
                        status: "quarantined".to_string(),
                        warning: None,
                        error: Some(format!("STEP_3_FAIL: {}", e)),
                    };
                }
            }
        }

        // Step 6: Emission calculation
        let (emission_factor, factor_source) = Self::get_emission_factor(&target);
        let tco2e = (canonical_value * emission_factor) / 1000.0;

        // Validation against physical ranges
        let range_result = RangeGuard::check(&target, emission_factor / 1000.0);
        
        let (status, warning, error) = match range_result {
            RangeCheckResult::Ok => ("clean".to_string(), None, None),
            RangeCheckResult::BestEffort(w) => ("best_effort".to_string(), Some(w), None),
            RangeCheckResult::HardStop(e) => ("quarantined".to_string(), None, Some(format!("STEP_6_FAIL: {}", e))),
        };

        MappingResult {
            esrs_target: target,
            raw_header: raw_header.to_string(),
            raw_value: raw_value.to_string(),
            canonical_value,
            canonical_unit: final_unit_str,
            tco2e,
            emission_factor,
            factor_source,
            confidence: 1.0,
            status,
            warning,
            error,
        }
    }
}
