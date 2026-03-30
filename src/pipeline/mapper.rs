// AI-assisted field mapping and transformation
use crate::models::MappingResult;
use crate::pipeline::validator::{CanonicalUnit, RangeCheckResult, RangeGuard, UnitValidator};

pub struct Mapper;

struct DictionaryEntry {
    esrs_target: &'static str,
    canonical_unit: CanonicalUnit,
    emission_factor: f64, // tCO2e per canonical unit
    confidence: f64,
    source: &'static str,
}

fn normalize_header(raw: &str) -> String {
    let mut s = raw.to_lowercase();
    s = s.replace('(', " ")
         .replace(')', " ")
         .replace('[', " ")
         .replace(']', " ")
         .replace('{', " ")
         .replace('}', " ")
         .replace('/', " ");
    
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_german_float(raw: &str) -> Option<f64> {
    let s = raw.trim().replace('.', "").replace(',', ".");
    s.parse::<f64>().ok()
}

fn lookup_dictionary(normalized_header: &str) -> Option<DictionaryEntry> {
    match normalized_header {
        "stromverbrauch" | "strom" | "stromverbrauch in mwh" | "stromverbrauch in kwh" | "electricity" | "power consumption" => {
            Some(DictionaryEntry {
                esrs_target: "E1-6_Scope2_Electricity",
                canonical_unit: CanonicalUnit::KiloWattHour,
                emission_factor: 0.000233,
                confidence: 1.0,
                source: "DEFRA_2024_DE",
            })
        }
        "erdgas" | "gasverbrauch" | "natural gas" | "gas consumption" | "erdgasverbrauch" => {
            Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_NaturalGas",
                canonical_unit: CanonicalUnit::KiloWattHour,
                emission_factor: 0.000203,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "diesel" | "dieselverbrauch" | "diesel consumption" => {
            Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_Diesel",
                canonical_unit: CanonicalUnit::Liter,
                emission_factor: 0.002640,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "benzin" | "benzinverbrauch" | "petrol" | "petrol consumption" => {
            Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_Petrol",
                canonical_unit: CanonicalUnit::Liter,
                emission_factor: 0.002310,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "heizöl" | "heizoel" | "heating oil" | "heizölverbrauch" => {
            Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_Heating_Oil",
                canonical_unit: CanonicalUnit::Liter,
                emission_factor: 0.002540,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "straßentransport" | "lkw" | "road freight" | "strassentransport" => {
            Some(DictionaryEntry {
                esrs_target: "E1-3_Scope3_Road_Freight",
                canonical_unit: CanonicalUnit::TonneKilometer,
                emission_factor: 0.000107,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "luftfracht" | "air freight" | "flugzeug transport" => {
            Some(DictionaryEntry {
                esrs_target: "E1-3_Scope3_Air_Freight",
                canonical_unit: CanonicalUnit::TonneKilometer,
                emission_factor: 0.000602,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "schienentransport" | "bahn" | "rail freight" => {
            Some(DictionaryEntry {
                esrs_target: "E1-3_Scope3_Rail_Freight",
                canonical_unit: CanonicalUnit::TonneKilometer,
                emission_factor: 0.000028,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        "kältemittel" | "kaeltemittel" | "refrigerants" | "kältemittelverbrauch" => {
            Some(DictionaryEntry {
                esrs_target: "E1-2_Scope1_Refrigerants",
                canonical_unit: CanonicalUnit::Kilogram,
                emission_factor: 0.675000,
                confidence: 1.0,
                source: "IPCC_AR6",
            })
        }
        "abfall deponie" | "deponieabfall" | "waste landfill" | "abfall" => {
            Some(DictionaryEntry {
                esrs_target: "E1-3_Scope3_Waste_Landfill",
                canonical_unit: CanonicalUnit::Kilogram,
                emission_factor: 0.000587,
                confidence: 1.0,
                source: "DEFRA_2024",
            })
        }
        _ => None,
    }
}

impl Mapper {
    pub fn process(
        raw_header: &str,
        raw_value: &str,
        input_unit_raw: &str,
    ) -> MappingResult {
        let normalized = normalize_header(raw_header);
        
        let dict = match lookup_dictionary(&normalized) {
            Some(d) => d,
            None => return MappingResult {
                esrs_target: "unknown".to_string(),
                raw_header: raw_header.to_string(),
                raw_value: raw_value.to_string(),
                canonical_value: 0.0,
                canonical_unit: "unknown".to_string(),
                tco2e: 0.0,
                emission_factor: 0.0,
                factor_source: "none".to_string(),
                confidence: 0.0,
                status: "quarantined".to_string(),
                warning: None,
                error: Some("Unknown header - requires AI mapping".to_string()),
            },
        };

        let parsed_value = match parse_german_float(raw_value) {
            Some(v) => v,
            None => return MappingResult {
                esrs_target: dict.esrs_target.to_string(),
                raw_header: raw_header.to_string(),
                raw_value: raw_value.to_string(),
                canonical_value: 0.0,
                canonical_unit: dict.canonical_unit.to_string(),
                tco2e: 0.0,
                emission_factor: dict.emission_factor,
                factor_source: dict.source.to_string(),
                confidence: 0.0,
                status: "quarantined".to_string(),
                warning: None,
                error: Some("Could not parse numeric value".to_string()),
            },
        };

        let input_unit = match UnitValidator::parse(input_unit_raw) {
            Ok(u) => u,
            Err(e) => return MappingResult {
                esrs_target: dict.esrs_target.to_string(),
                raw_header: raw_header.to_string(),
                raw_value: raw_value.to_string(),
                canonical_value: parsed_value,
                canonical_unit: "unknown".to_string(),
                tco2e: 0.0,
                emission_factor: dict.emission_factor,
                factor_source: dict.source.to_string(),
                confidence: 0.0,
                status: "quarantined".to_string(),
                warning: None,
                error: Some(format!("{}", e)),
            },
        };

        if let Err(e) = UnitValidator::check_family_match(&input_unit, &dict.canonical_unit) {
            return MappingResult {
                esrs_target: dict.esrs_target.to_string(),
                raw_header: raw_header.to_string(),
                raw_value: raw_value.to_string(),
                canonical_value: parsed_value,
                canonical_unit: input_unit.to_string(),
                tco2e: 0.0,
                emission_factor: dict.emission_factor,
                factor_source: dict.source.to_string(),
                confidence: 0.0,
                status: "quarantined".to_string(),
                warning: None,
                error: Some(format!("HARD ERROR: {}", e)),
            };
        }

        let canonical_value = UnitValidator::convert_to_base(parsed_value, &input_unit);
        let tco2e = canonical_value * dict.emission_factor;
        
        // Calculated tCO2e per unit for range check (it's essentially the emission factor in this case)
        let tco2e_per_unit = if canonical_value != 0.0 {
            tco2e / canonical_value
        } else {
            dict.emission_factor
        };

        let range_check = RangeGuard::check(dict.esrs_target, tco2e_per_unit);

        let mut res = MappingResult {
            esrs_target: dict.esrs_target.to_string(),
            raw_header: raw_header.to_string(),
            raw_value: raw_value.to_string(),
            canonical_value,
            canonical_unit: dict.canonical_unit.to_string(),
            tco2e,
            emission_factor: dict.emission_factor,
            factor_source: dict.source.to_string(),
            confidence: dict.confidence,
            status: "clean".to_string(),
            warning: None,
            error: None,
        };

        match range_check {
            RangeCheckResult::Ok => {}
            RangeCheckResult::BestEffort(msg) => {
                res.status = "best_effort".to_string();
                res.confidence = 0.75;
                res.warning = Some(msg);
            }
            RangeCheckResult::HardStop(msg) => {
                res.status = "quarantined".to_string();
                res.confidence = 0.0;
                res.error = Some(msg);
            }
        }

        res
    }
}
