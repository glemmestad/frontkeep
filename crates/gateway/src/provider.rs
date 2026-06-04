//! Model providers behind one interface (brief §4.2). Mock is fully exercised;
//! OpenAI and Anthropic are real HTTP adapters (hit live only when creds are
//! present); Bedrock is an adapter whose live path needs an AWS SigV4 signer
//! (documented follow-up, not wired into the OSS core build).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        ChatMessage {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        ChatMessage {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Downstream attribution: the owning project id, set by the gateway and
    /// forwarded to the provider (OpenAI `user`, Anthropic `metadata.user_id`) so
    /// the downstream's own logs/spend carry the project. Never caller-supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub model: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("transport error: {0}")]
    Http(String),
    #[error("provider api error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("decode error: {0}")]
    Decode(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    /// `route_model` is the provider-native model id (e.g. `gpt-4o`).
    async fn chat(
        &self,
        route_model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, ProviderError>;
}

fn word_count(s: &str) -> u32 {
    s.split_whitespace().count() as u32
}

/// Deterministic provider for tests and credential-free routing/guardrail/budget
/// proofs. Token counts are a stable function of the input.
pub struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn chat(
        &self,
        route_model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        let prompt_tokens: u32 = req.messages.iter().map(|m| word_count(&m.content)).sum();
        let last = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let content = format!("[mock:{route_model}] echo: {last}");
        Ok(ChatResponse {
            completion_tokens: word_count(&content),
            prompt_tokens,
            content,
            model: route_model.to_string(),
        })
    }
}

#[derive(Deserialize)]
struct OpenAiResp {
    choices: Vec<OpenAiChoice>,
    usage: OpenAiUsage,
}
#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsg,
}
#[derive(Deserialize)]
struct OpenAiMsg {
    content: String,
}
#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Default request path: standard OpenAI Chat Completions.
const DEFAULT_CHAT_PATH: &str = "/v1/chat/completions";

/// OpenAI Chat Completions adapter — and, via `chat_path`, any OpenAI-shaped
/// upstream whose request path differs (LiteLLM, vLLM, Databricks Model Serving).
/// The control plane (tokens/cost/policy/audit) is identical; only the upstream
/// URL changes, which is why these are plug-in *manifests*, not core providers.
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    /// Path appended to `base_url`. A `{model}` placeholder is substituted with the
    /// route model — so an endpoint-in-path upstream (Databricks
    /// `/serving-endpoints/{model}/invocations`) is expressible without bespoke code.
    chat_path: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        OpenAiProvider {
            api_key: api_key.into(),
            base_url: "https://api.openai.com".into(),
            chat_path: DEFAULT_CHAT_PATH.into(),
            client: reqwest::Client::new(),
        }
    }
    pub fn with_base_url(mut self, b: impl Into<String>) -> Self {
        self.base_url = b.into();
        self
    }
    /// Override the request path (default `/v1/chat/completions`). May contain a
    /// `{model}` placeholder. An empty/None override keeps the default.
    pub fn with_chat_path(mut self, p: Option<String>) -> Self {
        if let Some(p) = p.filter(|s| !s.is_empty()) {
            self.chat_path = p;
        }
        self
    }
    fn url(&self, route_model: &str) -> String {
        let path = self.chat_path.replace("{model}", route_model);
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        route_model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        // Only send optional params when set: newer models (e.g. gpt-5) reject a
        // non-default temperature and use max_completion_tokens, so omitting these
        // lets the model apply its own defaults.
        let mut body = serde_json::json!({ "model": route_model, "messages": req.messages });
        if let Some(mt) = req.max_tokens {
            body["max_completion_tokens"] = serde_json::json!(mt);
        }
        if let Some(t) = req.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        // Downstream attribution: OpenAI/LiteLLM log + meter by `user`.
        if let Some(u) = &req.user {
            body["user"] = serde_json::json!(u);
        }
        let resp = self
            .client
            .post(self.url(route_model))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        let parsed: OpenAiResp = resp
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        Ok(ChatResponse {
            content,
            prompt_tokens: parsed.usage.prompt_tokens,
            completion_tokens: parsed.usage.completion_tokens,
            model: route_model.to_string(),
        })
    }
}

#[derive(Deserialize)]
struct AnthropicResp {
    content: Vec<AnthropicBlock>,
    usage: AnthropicUsage,
}
#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(default)]
    text: String,
}
#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic Messages adapter.
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    version: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        AnthropicProvider {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
            version: "2023-06-01".into(),
            client: reqwest::Client::new(),
        }
    }
    pub fn with_base_url(mut self, b: impl Into<String>) -> Self {
        self.base_url = b.into();
        self
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        route_model: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        let mut body = serde_json::json!({
            "model": route_model,
            "max_tokens": req.max_tokens.unwrap_or(1024),
            "messages": req.messages,
        });
        // Downstream attribution: Anthropic logs by metadata.user_id.
        if let Some(u) = &req.user {
            body["metadata"] = serde_json::json!({ "user_id": u });
        }
        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        let parsed: AnthropicResp = resp
            .json()
            .await
            .map_err(|e| ProviderError::Decode(e.to_string()))?;
        let content = parsed
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");
        Ok(ChatResponse {
            content,
            prompt_tokens: parsed.usage.input_tokens,
            completion_tokens: parsed.usage.output_tokens,
            model: route_model.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_is_deterministic_and_counts_tokens() {
        let p = MockProvider;
        let req = ChatRequest {
            model: "model:default/m".into(),
            messages: vec![ChatMessage::user("hello there friend")],
            max_tokens: None,
            temperature: None,
            user: None,
        };
        let a = p.chat("gpt-4o", &req).await.unwrap();
        let b = p.chat("gpt-4o", &req).await.unwrap();
        assert_eq!(a.content, b.content);
        assert_eq!(a.prompt_tokens, 3);
        assert!(a.content.contains("hello there friend"));
    }

    #[test]
    fn chat_path_defaults_to_openai_and_overrides_with_model_placeholder() {
        // Default: standard OpenAI chat completions.
        let oa = OpenAiProvider::new("k");
        assert_eq!(
            oa.url("gpt-5.1"),
            "https://api.openai.com/v1/chat/completions"
        );
        // Generic override → Databricks Model Serving (endpoint in the path), as a
        // pure manifest, no Databricks-specific provider code. Trailing slash safe.
        let dbx = OpenAiProvider::new("k")
            .with_base_url("https://x.cloud.databricks.com/")
            .with_chat_path(Some("/serving-endpoints/{model}/invocations".into()));
        assert_eq!(
            dbx.url("databricks-claude-sonnet"),
            "https://x.cloud.databricks.com/serving-endpoints/databricks-claude-sonnet/invocations"
        );
        // Empty override keeps the default.
        let keep = OpenAiProvider::new("k").with_chat_path(Some(String::new()));
        assert_eq!(keep.url("m"), "https://api.openai.com/v1/chat/completions");
    }
}
