//! Cost sources: how spend is attributed back to a project, decoupled from
//! provisioning. A service can be created via Terraform but billed via the
//! cloud's billing API — so a manifest's `cost.source.type` binds to a
//! [`CostSource`] independently of its `provisioner.connector`. Every source
//! filters on the immutable `project=<id>` tag/label that
//! [`ResourceContext::tags`](crate::ResourceContext::tags) stamps on every
//! resource.

mod aws_ce;
mod databricks;
mod exec;
mod flat;
pub mod forecast;
mod gateway;
mod litellm;
pub mod qa;
pub mod report;
pub mod rollup;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::{CostWindow, ProvisionError, ServiceCost};

pub use aws_ce::AwsCostExplorerSource;
pub use databricks::DatabricksCostSource;
pub use exec::ExecCostSource;
pub use flat::FlatSource;
pub use forecast::{forecast_eom, linreg, Fit, Forecast};
pub use gateway::GatewaySource;
pub use litellm::LiteLlmCostSource;
pub use report::{
    build_tree, movers, tagged_report, CostNode, Mover, Movers, ProjectOverlay, TaggedReport,
};
pub use rollup::{
    AnomalyRow, CostRollupRepo, DimRow, ForecastRow, ProjectFact, RollupDim, RollupRow,
};

/// A source of actual incurred cost for a project over a window.
#[async_trait]
pub trait CostSource: Send + Sync {
    fn name(&self) -> &str;
    async fn cost(
        &self,
        project_id: &str,
        window: &CostWindow,
    ) -> Result<ServiceCost, ProvisionError>;
}

/// A source with no live billing feed: free/unmetered services, or the `flat`
/// model where the manifest estimate already stands in for the cost. Reports no
/// actual so a reader can tell a real $0 from "not measured here".
pub struct NoneSource {
    label: String,
}

impl NoneSource {
    pub fn new(label: impl Into<String>) -> Self {
        NoneSource {
            label: label.into(),
        }
    }
}

#[async_trait]
impl CostSource for NoneSource {
    fn name(&self) -> &str {
        &self.label
    }
    async fn cost(&self, _p: &str, _w: &CostWindow) -> Result<ServiceCost, ProvisionError> {
        Ok(ServiceCost {
            backend: self.label.clone(),
            actual_usd: None,
            source: self.label.clone(),
        })
    }
}

/// A declared-but-unconfigured source (cloud billing seam awaiting operator
/// creds). The shape is real; it reports "unconfigured" until wired.
pub struct UnconfiguredSource {
    label: String,
}

impl UnconfiguredSource {
    pub fn new(label: impl Into<String>) -> Self {
        UnconfiguredSource {
            label: label.into(),
        }
    }
}

#[async_trait]
impl CostSource for UnconfiguredSource {
    fn name(&self) -> &str {
        &self.label
    }
    async fn cost(&self, _p: &str, _w: &CostWindow) -> Result<ServiceCost, ProvisionError> {
        Ok(ServiceCost {
            backend: self.label.clone(),
            actual_usd: None,
            source: format!("{} unconfigured", self.label),
        })
    }
}

/// Cost sources keyed by `cost.source.type`. Built in `build_provision` from
/// operator config (per-cloud creds), with `none`/`free`/`flat` always present.
#[derive(Clone)]
pub struct CostSourceRegistry {
    sources: HashMap<String, Arc<dyn CostSource>>,
}

impl Default for CostSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CostSourceRegistry {
    pub fn new() -> Self {
        let mut sources: HashMap<String, Arc<dyn CostSource>> = HashMap::new();
        sources.insert("none".into(), Arc::new(NoneSource::new("none")));
        sources.insert("free".into(), Arc::new(NoneSource::new("free")));
        sources.insert("flat".into(), Arc::new(FlatSource));
        CostSourceRegistry { sources }
    }

    pub fn register(&mut self, key: impl Into<String>, src: Arc<dyn CostSource>) {
        self.sources.insert(key.into(), src);
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn CostSource>> {
        self.sources.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn none_flat_and_unconfigured_report_no_actual() {
        let w = CostWindow {
            start: "2026-06-01".into(),
            end: "2026-07-01".into(),
        };
        for src in [
            Arc::new(NoneSource::new("none")) as Arc<dyn CostSource>,
            Arc::new(FlatSource),
            Arc::new(UnconfiguredSource::new("gcp-billing")),
        ] {
            assert!(src.cost("p", &w).await.unwrap().actual_usd.is_none());
        }
    }

    #[test]
    fn default_registry_has_the_free_sources() {
        let r = CostSourceRegistry::new();
        assert!(r.get("none").is_some());
        assert!(r.get("free").is_some());
        assert!(r.get("flat").is_some());
        assert!(r.get("aws-cost-explorer").is_none());
    }
}
