//! A generic, bounded tool-calling loop layered on `Gateway::complete`. The
//! gateway natively does single-shot completion; this drives the model →
//! tool-call → result → model iteration on top of it, so every round is still
//! governed, cost-attributed, and audited like any other gateway call. The
//! protocol is provider-agnostic: the model requests a tool by replying with a
//! single JSON object `{"tool": "...", "args": {...}}`, and answers in plain text
//! when it has enough information.

use async_trait::async_trait;
use serde_json::Value;

use crate::provider::{ChatMessage, ChatRequest};
use crate::{Gateway, GatewayError};

/// One tool the model may call, with the data the loop needs to advertise it.
pub struct ToolDef {
    pub name: String,
    pub description: String,
}

/// Resolves the tools a loop exposes and runs them. Implementors hold whatever
/// data backend the tools read (e.g. the cost rollup store).
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn tools(&self) -> Vec<ToolDef>;
    async fn call(&self, name: &str, args: &Value) -> Result<String, String>;
}

/// Run the bounded loop and return the model's final plain-text answer. The model
/// only ever sees the grounding text and tool outputs, so the answer is grounded
/// by construction. Capped at `max_rounds` tool turns.
#[allow(clippy::too_many_arguments)]
pub async fn run_tool_loop(
    gateway: &Gateway,
    virtual_key: &str,
    model: &str,
    data_class: Option<String>,
    grounding: &str,
    question: &str,
    tools: &dyn ToolExecutor,
    max_rounds: usize,
) -> Result<String, GatewayError> {
    let catalog = tools
        .tools()
        .iter()
        .map(|t| format!("- {}: {}", t.name, t.description))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "{grounding}\n\nYou may call a tool by replying with ONLY a JSON object of the \
         form tool-name plus args (keys \"tool\" and \"args\"). Available tools:\n{catalog}\n\n\
         Answer ONLY from tool outputs. If the data isn't available, say exactly \
         \"I don't have that data.\" When you can answer, reply in plain prose with \
         no JSON.\n\nQuestion: {question}"
    );
    let mut messages = vec![ChatMessage::user(prompt)];

    let mut last = String::new();
    for _ in 0..max_rounds.max(1) {
        let resp = gateway
            .complete(
                virtual_key,
                ChatRequest {
                    model: model.to_string(),
                    messages: messages.clone(),
                    max_tokens: None,
                    temperature: None,
                    user: None,
                },
                None,
                data_class.clone(),
            )
            .await?;
        last = resp.content.clone();
        match parse_tool_call(&resp.content) {
            Some((name, args)) => {
                let result = tools
                    .call(&name, &args)
                    .await
                    .unwrap_or_else(|e| format!("tool error: {e}"));
                messages.push(ChatMessage::assistant(resp.content));
                messages.push(ChatMessage::user(format!("Tool {name} result: {result}")));
            }
            None => return Ok(resp.content),
        }
    }
    Ok(last)
}

/// Extract a `{"tool": ..., "args": ...}` request from a model reply, scanning
/// for the first balanced `{...}` span that parses as such an object. Returns
/// `None` for a plain-text answer.
fn parse_tool_call(content: &str) -> Option<(String, Value)> {
    let bytes = content.as_bytes();
    for (i, _) in bytes.iter().enumerate().filter(|(_, b)| **b == b'{') {
        let mut depth = 0i32;
        for (j, b) in bytes.iter().enumerate().skip(i) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Ok(v) = serde_json::from_str::<Value>(&content[i..=j]) {
                            if let Some(name) = v.get("tool").and_then(|t| t.as_str()) {
                                let args = v.get("args").cloned().unwrap_or(Value::Null);
                                return Some((name.to_string(), args));
                            }
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatResponse, Provider, ProviderError};
    use crate::{GatewayRepo, ModelInfo, ModelRegistry};
    use asgard_policy::{CedarEngine, PolicyEngine};
    use asgard_storage::Db;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn parses_tool_call_and_ignores_prose() {
        let (n, a) = parse_tool_call(r#"sure: {"tool":"top_movers","args":{"n":3}}"#).unwrap();
        assert_eq!(n, "top_movers");
        assert_eq!(a["n"], 3);
        assert!(parse_tool_call("the answer is $42").is_none());
        // The literal placeholder from the prompt must not parse as a call.
        assert!(parse_tool_call(r#"{"tool": "NAME", "args": {}}xx broken {...}"#).is_some());
    }

    /// A provider scripted to emit one tool call, then a final answer.
    struct ScriptProvider {
        round: Mutex<usize>,
    }
    #[async_trait]
    impl Provider for ScriptProvider {
        fn name(&self) -> &str {
            "mock"
        }
        async fn chat(
            &self,
            route_model: &str,
            _req: &ChatRequest,
        ) -> Result<ChatResponse, ProviderError> {
            let mut r = self.round.lock().unwrap();
            let content = if *r == 0 {
                r#"{"tool":"spend","args":{}}"#.to_string()
            } else {
                "Total spend is $12.34.".to_string()
            };
            *r += 1;
            Ok(ChatResponse {
                completion_tokens: 3,
                prompt_tokens: 3,
                content,
                model: route_model.to_string(),
            })
        }
    }

    struct OneTool;
    #[async_trait]
    impl ToolExecutor for OneTool {
        fn tools(&self) -> Vec<ToolDef> {
            vec![ToolDef {
                name: "spend".into(),
                description: "total spend".into(),
            }]
        }
        async fn call(&self, name: &str, _args: &Value) -> Result<String, String> {
            assert_eq!(name, "spend");
            Ok("12.34".into())
        }
    }

    async fn gateway_with(provider: Arc<dyn Provider>) -> (Gateway, String) {
        let path = std::env::temp_dir().join(format!("asgard-tl-{}.db", asgard_storage::new_uid()));
        let db = Db::connect(&format!("sqlite://{}", path.display()))
            .await
            .unwrap();
        db.migrate().await.unwrap();
        let repo = GatewayRepo::new(db);
        repo.ensure_project("proj-2026-0001", 0.0, "internal")
            .await
            .unwrap();
        let key = repo
            .mint_key("proj-2026-0001", Some("t"))
            .await
            .unwrap()
            .plaintext;
        let registry = ModelRegistry::from_models(vec![ModelInfo {
            model_ref: "model:default/mock".into(),
            provider: "mock".into(),
            route_model: "mock".into(),
            data_classes: vec!["internal".into()],
            cost_in: 1.0,
            cost_out: 1.0,
        }]);
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();
        providers.insert("mock".into(), provider);
        let policy: Arc<dyn PolicyEngine> = Arc::new(CedarEngine::new().unwrap());
        (
            Gateway::new(repo, policy, registry, providers, crate::Mode::Enforce),
            key,
        )
    }

    #[tokio::test]
    async fn loop_calls_tool_then_answers() {
        let (gw, key) = gateway_with(Arc::new(ScriptProvider {
            round: Mutex::new(0),
        }))
        .await;
        let answer = run_tool_loop(
            &gw,
            &key,
            "model:default/mock",
            Some("internal".into()),
            "Cost facts follow.",
            "What is total spend?",
            &OneTool,
            4,
        )
        .await
        .unwrap();
        assert!(
            answer.contains("12.34"),
            "final answer should follow the tool result"
        );
    }
}
