//! Unified LLM provider abstraction.
//!
//! Crush diagnosis defaults to **Gemini** (Google's free tier) and falls back
//! to **Anthropic** when only `ANTHROPIC_API_KEY` is set. End users bring their
//! own key (BYOK) — Crush never ships a key. Resolution order:
//!   1. `GEMINI_API_KEY` / `GOOGLE_API_KEY`  -> Gemini   (preferred, free)
//!   2. `ANTHROPIC_API_KEY`                  -> Anthropic
//! The model can be overridden with `CRUSH_AI_MODEL`.

use anyhow::{anyhow, Result};
use crate::ai_client::DiagnosisResult;

/// Default models, chosen for fast/cheap error triage. Overridable via env.
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.5-flash";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-6";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Gemini,
    Anthropic,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::Anthropic => "anthropic",
        }
    }
}

/// A configured LLM endpoint (provider + model + key). Cheap to clone; the key
/// is kept private so it is never accidentally logged via `Debug`.
#[derive(Clone)]
pub struct LlmClient {
    pub provider: Provider,
    pub model: String,
    api_key: String,
}

// Custom Debug that never prints the key.
impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl LlmClient {
    pub fn gemini(api_key: String) -> Self {
        Self {
            provider: Provider::Gemini,
            model: model_override().unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string()),
            api_key,
        }
    }

    pub fn anthropic(api_key: String) -> Self {
        Self {
            provider: Provider::Anthropic,
            model: model_override().unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string()),
            api_key,
        }
    }

    /// Infer the provider from the shape of a user-supplied key. Anthropic keys
    /// start with `sk-ant`; everything else (incl. Google `AIza…`) is Gemini.
    pub fn from_key(key: String) -> Self {
        if key.starts_with("sk-ant") {
            Self::anthropic(key)
        } else {
            Self::gemini(key)
        }
    }

    /// Resolve a client from the environment (BYOK). Gemini is preferred.
    pub fn from_env() -> Option<Self> {
        for var in ["GEMINI_API_KEY", "GOOGLE_API_KEY"] {
            if let Ok(k) = std::env::var(var) {
                if !k.trim().is_empty() {
                    return Some(Self::gemini(k));
                }
            }
        }
        if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
            if !k.trim().is_empty() {
                return Some(Self::anthropic(k));
            }
        }
        None
    }

    /// Human-readable provider+model label for logging/UX (no key).
    pub fn label(&self) -> String {
        format!("{} ({})", self.provider.name(), self.model)
    }

    /// Single-shot completion: system + user prompt -> assistant text.
    pub async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        match self.provider {
            Provider::Gemini => self.complete_gemini(system, user, max_tokens).await,
            Provider::Anthropic => self.complete_anthropic(system, user, max_tokens).await,
        }
    }

    async fn complete_gemini(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );
        let body = serde_json::json!({
            "system_instruction": { "parts": [{ "text": system }] },
            "contents": [{ "role": "user", "parts": [{ "text": user }] }],
            "generationConfig": {
                "maxOutputTokens": max_tokens,
                "temperature": 0.2,
                "responseMimeType": "application/json"
            }
        });

        let client = reqwest::Client::new();
        let res = client
            .post(&url)
            .header("x-goog-api-key", &self.api_key) // key in header, never the URL
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API error {}: {}", status, text));
        }

        let v: serde_json::Value = res.json().await?;
        let text = v["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Gemini response had no text content"))?;
        Ok(text.to_string())
    }

    async fn complete_anthropic(&self, system: &str, user: &str, max_tokens: u32) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": [{ "role": "user", "content": user }]
        });

        let client = reqwest::Client::new();
        let res = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error {}: {}", status, text));
        }

        let v: serde_json::Value = res.json().await?;
        let text = v["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Anthropic response had no text content"))?;
        Ok(text.to_string())
    }

    /// High-level: diagnose an error and return a structured result. The model
    /// is asked for raw JSON; we defensively extract the first JSON object in
    /// case it wraps the answer in prose or markdown fences.
    pub async fn diagnose_error(
        &self,
        error_json: &str,
        source_context: Option<&str>,
        project_summary: Option<&str>,
    ) -> Result<DiagnosisResult> {
        let system = "You are an expert debugging engineer. Analyze the error and respond with ONLY a raw JSON object with exactly these keys: \
root_cause (string), fix_description (string), proposed_patch (string or null; a unified diff when possible), \
auto_fixable (boolean), confidence (number between 0 and 1). No markdown, no commentary.";
        let user = format!(
            "Error details:\n{}\n\nSource context:\n{}\n\nProject summary:\n{}",
            error_json,
            source_context.unwrap_or("(none)"),
            project_summary.unwrap_or("(none)")
        );

        let raw = self.complete(system, &user, 2048).await?;
        let json = extract_json(&raw)
            .ok_or_else(|| anyhow!("model response contained no JSON object"))?;
        let diag: DiagnosisResult = serde_json::from_str(json)?;
        Ok(diag)
    }
}

fn model_override() -> Option<String> {
    std::env::var("CRUSH_AI_MODEL").ok().filter(|s| !s.trim().is_empty())
}

/// Slice out the first balanced-looking JSON object (first `{` .. last `}`).
fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}
