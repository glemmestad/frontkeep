//! LiteLLM cost source: per-project spend read back from a LiteLLM proxy. For
//! each `litellm-key` resource the project provisioned, this reads the key's
//! cumulative spend via `/key/info?key_alias=…` and sums them. The daily rollup
//! loop converts the cumulative figure into a per-day delta, exactly like the
//! cloud billing sources.
//!
//! `/key/info` is chosen over `/spend/tags` (enterprise-gated) and `/spend/logs`
//! (paginated, not project-keyed): the alias is the stable per-project handle the
//! connector stamped at mint time. Any failure reports `actual_usd: None` (the
//! estimate stands in) rather than erroring the rollup; no keys is a real $0.

use async_trait::async_trait;
use serde_json::Value;

use super::CostSource;
use crate::{CostWindow, ProvisionError, ProvisionRepo, ServiceCost};

pub struct LiteLlmCostSource {
    base_url: String,
    master_key: String,
    repo: ProvisionRepo,
    client: reqwest::Client,
}

impl LiteLlmCostSource {
    pub fn new(
        base_url: impl Into<String>,
        master_key: impl Into<String>,
        repo: ProvisionRepo,
    ) -> Self {
        LiteLlmCostSource {
            base_url: base_url.into(),
            master_key: master_key.into(),
            repo,
            client: reqwest::Client::new(),
        }
    }

    async fn key_spend(&self, alias: &str) -> Option<f64> {
        let url = format!("{}/key/info", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(url)
            .query(&[("key_alias", alias)])
            .header("Authorization", format!("Bearer {}", self.master_key))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let payload: Value = resp.json().await.ok()?;
        parse_key_spend(&payload)
    }
}

/// Extract cumulative spend from a `/key/info` response. LiteLLM nests the key
/// record under `info` (`{"info": {"spend": 1.23, …}}`); a flat `spend` is
/// accepted too for resilience across versions.
fn parse_key_spend(payload: &Value) -> Option<f64> {
    payload
        .pointer("/info/spend")
        .or_else(|| payload.get("spend"))
        .and_then(|v| v.as_f64())
}

#[async_trait]
impl CostSource for LiteLlmCostSource {
    fn name(&self) -> &str {
        "litellm"
    }

    async fn cost(
        &self,
        project_id: &str,
        _window: &CostWindow,
    ) -> Result<ServiceCost, ProvisionError> {
        let unavailable = |why: String| ServiceCost {
            backend: "litellm".into(),
            actual_usd: None,
            source: format!("litellm unavailable: {why}"),
        };
        let records = match self.repo.list_by_project(project_id).await {
            Ok(r) => r,
            Err(e) => return Ok(unavailable(e.to_string())),
        };
        let aliases: Vec<String> = records
            .iter()
            .filter(|r| r.rtype == "litellm-key")
            .filter_map(|r| {
                r.outputs
                    .get("key_alias")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        if aliases.is_empty() {
            return Ok(ServiceCost {
                backend: "litellm".into(),
                actual_usd: Some(0.0),
                source: "litellm".into(),
            });
        }
        let mut total = 0.0;
        for alias in &aliases {
            match self.key_spend(alias).await {
                Some(s) => total += s,
                None => return Ok(unavailable(format!("key/info for {alias}"))),
            }
        }
        Ok(ServiceCost {
            backend: "litellm".into(),
            actual_usd: Some(total),
            source: "litellm".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_nested_and_flat_spend() {
        assert_eq!(
            parse_key_spend(&json!({ "info": { "spend": 12.5 } })),
            Some(12.5)
        );
        assert_eq!(parse_key_spend(&json!({ "spend": 3.0 })), Some(3.0));
        assert_eq!(parse_key_spend(&json!({ "key": "sk-x" })), None);
    }
}
