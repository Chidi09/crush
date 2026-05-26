use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisRequest {
    pub error_json: String,
    pub source_context: Option<String>,
    pub project_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisResult {
    pub root_cause: String,
    pub fix_description: String,
    pub proposed_patch: Option<String>,
    pub auto_fixable: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct AiClient {
    pub api_key: String,
}

impl AiClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn diagnose(&self, req: DiagnosisRequest) -> Result<DiagnosisResult> {
        let client = reqwest::Client::new();
        let system_prompt = "You are an expert debugger. Analyze the error and return a JSON object with: {root_cause, fix_description, proposed_patch, auto_fixable, confidence}. Ensure proposed_patch contains only standard diff format or code changes. Return ONLY valid raw JSON.";
        let user_prompt = format!(
            "Analyze this error and return JSON: {{root_cause, fix_description, proposed_patch, auto_fixable, confidence}}.\nError details:\n{}\nSource Context:\n{:?}\nProject Summary:\n{:?}",
            req.error_json, req.source_context, req.project_summary
        );

        let body = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 2048,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_prompt}
            ]
        });

        let res = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let err_text = res.text().await?;
            return Err(anyhow!("Anthropic API error: {}", err_text));
        }

        let response_json: serde_json::Value = res.json().await?;
        let text = response_json.get("content")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Failed to parse response text from Claude"))?;

        // Extract JSON block if it's wrapped in markdown backticks
        let clean_json = if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                &text[start..=end]
            } else {
                text
            }
        } else {
            text
        };

        let diagnosis: DiagnosisResult = serde_json::from_str(clean_json)?;
        Ok(diagnosis)
    }

    pub async fn diagnose_streaming<F>(&self, req: DiagnosisRequest, callback: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        // For simplicity and fallback, call the non-streaming endpoint and feed callback
        let result = self.diagnose(req).await?;
        let text = format!(
            "Root Cause:\n{}\n\nFix Description:\n{}\n\nProposed Patch:\n{:?}\n",
            result.root_cause, result.fix_description, result.proposed_patch
        );
        for token in text.split_whitespace() {
            callback(&format!("{} ", token));
        }
        Ok(())
    }
}
