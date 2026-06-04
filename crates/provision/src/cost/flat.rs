//! Flat-rate source: the manifest's `estimated_monthly_usd` already stands in as
//! the cost (a fixed monthly fee), so there is no separate measured actual to
//! pull. The estimate is reported in `ProjectInfraCost::estimated_monthly_usd`;
//! this source reports no distinct actual.

use async_trait::async_trait;

use super::CostSource;
use crate::{CostWindow, ProvisionError, ServiceCost};

pub struct FlatSource;

#[async_trait]
impl CostSource for FlatSource {
    fn name(&self) -> &str {
        "flat"
    }
    async fn cost(&self, _p: &str, _w: &CostWindow) -> Result<ServiceCost, ProvisionError> {
        Ok(ServiceCost {
            backend: "flat".into(),
            actual_usd: None,
            source: "flat (estimate)".into(),
        })
    }
}
