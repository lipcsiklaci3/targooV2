use serde::{Deserialize, Serialize};
use serde_json::json;
use reqwest::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqMappingResult {
    pub esrs_target: String,
    pub canonical_unit: String,
    pub conversion_factor: f64,
    pub confidence: f64,
    pub reasoning: String,
}

pub struct GroqClient {
    api_key: String,
    client: Client,
}

impl GroqClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub async fn map_header(&self, raw_header: &str, language: &str, model: &str) -> Result<GroqMappingResult, String> {
        let system_prompt = "You are an expert ESG data taxonomy mapper specializing in ESRS and SFDR standards. \
            Your task is to map raw column headers from ESG data files to standardized ESRS targets. \
            You must respond with valid JSON only, no explanations, no markdown.";

        let user_prompt = format!(
            "Map this ESG data column header to the correct ESRS target.\n\n\
            Header: '{raw_header}'\n\
            Language: {language}\n\n\
            Valid ESRS targets:\n\
            - E1-6_Scope2_Electricity (unit: kWh)\n\
            - E1-1_Scope1_NaturalGas (unit: kWh)\n\
            - E1-1_Scope1_Diesel (unit: liter)\n\
            - E1-1_Scope1_Petrol (unit: liter)\n\
            - E1-1_Scope1_Heating_Oil (unit: liter)\n\
            - E1-3_Scope3_Road_Freight (unit: tkm)\n\
            - E1-3_Scope3_Air_Freight (unit: tkm)\n\
            - E1-3_Scope3_Rail_Freight (unit: tkm)\n\
            - E1-2_Scope1_Refrigerants (unit: kg)\n\
            - E1-3_Scope3_Waste_Landfill (unit: kg)\n\n\
            Respond with this exact JSON structure:\n\
            {{\n  \"esrs_target\": \"E1-6_Scope2_Electricity\",\n  \"canonical_unit\": \"kWh\",\n  \"conversion_factor\": 1.0,\n  \"confidence\": 0.8,\n  \"reasoning\": \"brief explanation\"\n}}\n\n\
            If you cannot map this header with confidence > 0.5, set confidence to 0.3 and esrs_target to \"UNKNOWN\"."
        );

        let payload = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 500,
            "temperature": 0.1
        });

        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("Groq API error: {}", response.status()));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from Groq response")?;

        // Handle potential markdown code blocks
        let clean_content = content.trim_matches('`').trim_start_matches("json").trim();
        
        let mut result: GroqMappingResult = serde_json::from_str(clean_content)
            .map_err(|e| format!("Failed to parse Groq response JSON: {}. Raw: {}", e, clean_content))?;

        if result.confidence < 0.5 {
            result.confidence = 0.3;
            result.esrs_target = "UNKNOWN".to_string();
        }

        Ok(result)
    }

    pub async fn map_with_fallback(&self, raw_header: &str, language: &str) -> GroqMappingResult {
        // Try gemma2-9b-it first
        match self.map_header(raw_header, language, "gemma2-9b-it").await {
            Ok(res) => res,
            Err(_) => {
                // Retry with llama3-8b-8192
                match self.map_header(raw_header, language, "llama3-8b-8192").await {
                    Ok(res) => res,
                    Err(_) => GroqMappingResult {
                        esrs_target: "UNKNOWN".to_string(),
                        canonical_unit: "UNKNOWN".to_string(),
                        conversion_factor: 1.0,
                        confidence: 0.0,
                        reasoning: "All AI models failed".to_string(),
                    },
                }
            }
        }
    }
}
