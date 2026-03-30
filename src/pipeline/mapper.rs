// ESG Data Mapping Logic
use crate::models::MappingResult;
use crate::pipeline::groq::GroqClient;

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
        match normalized_header {
            "strom" | "electricity" | "power" | "electrical energy" => Some(DictionaryEntry {
                esrs_target: "E1-6_Scope2_Electricity".to_string(),
                canonical_unit: "kWh".to_string(),
                conversion_factor: 1.0,
            }),
            "erdgas" | "natural gas" | "gas consumption" => Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_NaturalGas".to_string(),
                canonical_unit: "kWh".to_string(),
                conversion_factor: 1.0,
            }),
            "diesel" | "diesel fuel" | "dieselverbrauch" => Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_Diesel".to_string(),
                canonical_unit: "liter".to_string(),
                conversion_factor: 1.0,
            }),
            "benzin" | "petrol" | "gasoline" => Some(DictionaryEntry {
                esrs_target: "E1-1_Scope1_Petrol".to_string(),
                canonical_unit: "liter".to_string(),
                conversion_factor: 1.0,
            }),
            _ => None,
        }
    }

    fn get_emission_factor(esrs_target: &str) -> (f64, String) {
        match esrs_target {
            "E1-6_Scope2_Electricity" => (0.354, "IEA 2023 Germany".to_string()),
            "E1-1_Scope1_NaturalGas" => (0.202, "DEFRA 2024".to_string()),
            "E1-1_Scope1_Diesel" => (2.68, "DEFRA 2024".to_string()),
            "E1-1_Scope1_Petrol" => (2.31, "DEFRA 2024".to_string()),
            _ => (0.0, "Unknown".to_string()),
        }
    }

    pub async fn process(
        raw_header: &str,
        raw_value: &str,
        input_unit_raw: &str,
        groq_client: Option<&GroqClient>,
    ) -> MappingResult {
        let normalized_header = Self::normalize(raw_header);
        let mut status_source = "dictionary".to_string();
        let mut confidence = 1.0;

        // Step 1: Check Dictionary
        let (target, unit, factor) = match Self::lookup_dictionary(&normalized_header) {
            Some(entry) => (entry.esrs_target, entry.canonical_unit, entry.conversion_factor),
            None => {
                // Step 2: AI Fallback
                if let Some(groq) = groq_client {
                    let groq_res = groq.map_with_fallback(&normalized_header, "DE").await;
                    if groq_res.esrs_target == "UNKNOWN" {
                        return MappingResult {
                            esrs_target: "UNKNOWN".to_string(),
                            raw_header: raw_header.to_string(),
                            raw_value: raw_value.to_string(),
                            canonical_value: 0.0,
                            canonical_unit: "UNKNOWN".to_string(),
                            tco2e: 0.0,
                            emission_factor: 0.0,
                            factor_source: "None".to_string(),
                            confidence: groq_res.confidence,
                            status: "quarantined".to_string(),
                            warning: None,
                            error: Some(format!("AI Mapping failed: {}", groq_res.reasoning)),
                        };
                    }
                    if groq_res.confidence >= 0.5 {
                        status_source = "groq_ai".to_string();
                        confidence = groq_res.confidence;
                        (groq_res.esrs_target, groq_res.canonical_unit, groq_res.conversion_factor)
                    } else {
                        return MappingResult {
                            esrs_target: "UNKNOWN".to_string(),
                            raw_header: raw_header.to_string(),
                            raw_value: raw_value.to_string(),
                            canonical_value: 0.0,
                            canonical_unit: "UNKNOWN".to_string(),
                            tco2e: 0.0,
                            emission_factor: 0.0,
                            factor_source: "None".to_string(),
                            confidence: groq_res.confidence,
                            status: "quarantined".to_string(),
                            warning: None,
                            error: Some("Unknown header - AI confidence too low".to_string()),
                        };
                    }
                } else {
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
                        error: Some("Unknown header - AI unavailable".to_string()),
                    };
                }
            }
        };

        // Step 3: Parse and Calculate
        let val: f64 = raw_value.parse().unwrap_or(0.0);
        let canonical_value = val * factor;
        let (emission_factor, factor_source) = Self::get_emission_factor(&target);
        let tco2e = (canonical_value * emission_factor) / 1000.0;

        let status = if confidence >= 0.9 {
            "clean".to_string()
        } else {
            "best_effort".to_string()
        };

        MappingResult {
            esrs_target: target,
            raw_header: raw_header.to_string(),
            raw_value: raw_value.to_string(),
            canonical_value,
            canonical_unit: unit,
            tco2e,
            emission_factor,
            factor_source,
            confidence,
            status,
            warning: if status_source == "groq_ai" { Some("Mapped by AI - please verify".to_string()) } else { None },
            error: None,
        }
    }
}
