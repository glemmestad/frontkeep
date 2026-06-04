//! AWS Cost Explorer source: actual spend filtered to the `project=<id>`
//! cost-allocation tag stamped on every resource.
//!
//! Two operator prerequisites gate real numbers (both account-wide, both a
//! human's job): the `project` tag must be activated as a cost-allocation tag in
//! Billing, and Cost Explorer data lags real time by up to ~24h. Until then CE
//! reports `$0` or errors; we surface that honestly as `actual_usd: None` rather
//! than inventing a figure — the estimate stands in.

use async_trait::async_trait;
use serde_json::Value;
use tokio::process::Command;

use super::CostSource;
use crate::{CostWindow, ProvisionError, ServiceCost};

pub struct AwsCostExplorerSource {
    profile: Option<String>,
    /// When false the source is plan-only (no live CE call). Cost reads are
    /// non-destructive, so this is decoupled from the provisioner's execute flag.
    execute: bool,
    /// Whole-account mode: skip the `project` tag filter to read the entire bill
    /// (the tagged-% denominator). Registered separately as `account-total`.
    whole_account: bool,
    name: &'static str,
}

impl AwsCostExplorerSource {
    /// Per-project source: filters on the immutable `project=<id>` tag.
    pub fn new(profile: Option<String>, execute: bool) -> Self {
        AwsCostExplorerSource {
            profile,
            execute,
            whole_account: false,
            name: "aws-cost-explorer",
        }
    }

    /// Whole-account source: the un-tag-filtered bill, for the tagged-% denominator.
    pub fn account_total(profile: Option<String>, execute: bool) -> Self {
        AwsCostExplorerSource {
            profile,
            execute,
            whole_account: true,
            name: "account-total",
        }
    }
}

#[async_trait]
impl CostSource for AwsCostExplorerSource {
    fn name(&self) -> &str {
        self.name
    }

    async fn cost(
        &self,
        project_id: &str,
        window: &CostWindow,
    ) -> Result<ServiceCost, ProvisionError> {
        if !self.execute {
            return Ok(ServiceCost {
                backend: "aws".into(),
                actual_usd: None,
                source: "dry-run".into(),
            });
        }
        // Cost Explorer is a global service reached through us-east-1.
        let mut args = vec![
            "ce".to_string(),
            "get-cost-and-usage".into(),
            "--time-period".into(),
            format!("Start={},End={}", window.start, window.end),
            "--granularity".into(),
            "MONTHLY".into(),
            "--metrics".into(),
            "UnblendedCost".into(),
            "--region".into(),
            "us-east-1".into(),
        ];
        if !self.whole_account {
            let filter =
                serde_json::json!({ "Tags": { "Key": "project", "Values": [project_id] } })
                    .to_string();
            args.push("--filter".into());
            args.push(filter);
        }
        if let Some(p) = &self.profile {
            args.push("--profile".into());
            args.push(p.clone());
        }
        match run(&args).await {
            Ok(v) => {
                let total: f64 = v
                    .get("ResultsByTime")
                    .and_then(|r| r.as_array())
                    .map(|periods| {
                        periods
                            .iter()
                            .filter_map(|p| {
                                p.pointer("/Total/UnblendedCost/Amount")
                                    .and_then(|a| a.as_str())
                                    .and_then(|s| s.parse::<f64>().ok())
                            })
                            .sum()
                    })
                    .unwrap_or(0.0);
                Ok(ServiceCost {
                    backend: "aws".into(),
                    actual_usd: Some(total),
                    source: "aws-cost-explorer".into(),
                })
            }
            Err(ProvisionError::Backend(msg)) => Ok(ServiceCost {
                backend: "aws".into(),
                actual_usd: None,
                source: format!("aws-cost-explorer unavailable: {msg}"),
            }),
            Err(e) => Err(e),
        }
    }
}

async fn run(args: &[String]) -> Result<Value, ProvisionError> {
    let out = Command::new("aws")
        .args(args)
        .output()
        .await
        .map_err(|e| ProvisionError::Backend(format!("spawn aws: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let last = stderr
            .lines()
            .rev()
            .find(|l| l.contains("error"))
            .unwrap_or_else(|| stderr.trim());
        return Err(ProvisionError::Backend(format!(
            "aws ce failed: {}",
            last.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        return Ok(Value::Null);
    }
    Ok(serde_json::from_str(&stdout).unwrap_or(Value::Null))
}
