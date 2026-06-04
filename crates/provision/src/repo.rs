//! Persistence for provisioned resources. `project_id` is the join key that ties
//! infra spend into the same per-project rollup as model spend.

use asgard_storage::Db;
use serde::Serialize;
use sqlx::Row;

use crate::ProvisionError;

#[derive(Debug, Clone, Serialize)]
pub struct ProvisionedRecord {
    pub id: String,
    pub project_id: String,
    pub rtype: String,
    pub name: String,
    pub spec: serde_json::Value,
    pub outputs: serde_json::Value,
    pub tags: std::collections::BTreeMap<String, String>,
    pub est_monthly_usd: f64,
    pub state: String,
    pub backend: String,
    pub dry_run: bool,
    pub request_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct ProvisionRepo {
    db: Db,
}

impl ProvisionRepo {
    pub fn new(db: Db) -> Self {
        ProvisionRepo { db }
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub async fn record(&self, rec: &ProvisionedRecord) -> Result<(), ProvisionError> {
        sqlx::query(&self.db.q("INSERT INTO provisioned_resources \
             (id, project_id, rtype, name, spec, outputs, tags, est_monthly_usd, state, \
              backend, dry_run, request_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"))
        .bind(&rec.id)
        .bind(&rec.project_id)
        .bind(&rec.rtype)
        .bind(&rec.name)
        .bind(rec.spec.to_string())
        .bind(rec.outputs.to_string())
        .bind(serde_json::to_string(&rec.tags).unwrap_or_else(|_| "{}".into()))
        .bind(rec.est_monthly_usd)
        .bind(&rec.state)
        .bind(&rec.backend)
        .bind(rec.dry_run as i64)
        .bind(&rec.request_id)
        .bind(&rec.created_at)
        .bind(&rec.updated_at)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn list_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProvisionedRecord>, ProvisionError> {
        let rows = sqlx::query(&self.db.q(
            "SELECT id, project_id, rtype, name, spec, outputs, tags, est_monthly_usd, state, \
             backend, dry_run, request_id, created_at, updated_at \
             FROM provisioned_resources WHERE project_id = ? ORDER BY created_at DESC",
        ))
        .bind(project_id)
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(row_to_record).collect())
    }

    /// The resource already recorded for a workflow request, if any. Lets
    /// fulfillment be retried idempotently after a mid-operation failure without
    /// writing a duplicate row.
    pub async fn get_by_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ProvisionedRecord>, ProvisionError> {
        let row = sqlx::query(&self.db.q(
            "SELECT id, project_id, rtype, name, spec, outputs, tags, est_monthly_usd, state, \
             backend, dry_run, request_id, created_at, updated_at \
             FROM provisioned_resources WHERE request_id = ? ORDER BY created_at LIMIT 1",
        ))
        .bind(request_id)
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(row_to_record))
    }

    pub async fn get(&self, id: &str) -> Result<Option<ProvisionedRecord>, ProvisionError> {
        let row = sqlx::query(&self.db.q(
            "SELECT id, project_id, rtype, name, spec, outputs, tags, est_monthly_usd, state, \
             backend, dry_run, request_id, created_at, updated_at \
             FROM provisioned_resources WHERE id = ?",
        ))
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(row_to_record))
    }

    /// Set a resource's lifecycle `state` (free-text: `provisioned`, `suspending`,
    /// `suspended`, `destroying`, `destroyed`). Only `provisioned` accrues
    /// estimated cost, so a transition is what drops a resource off the bill.
    pub async fn mark_state(&self, id: &str, state: &str) -> Result<(), ProvisionError> {
        sqlx::query(
            &self
                .db
                .q("UPDATE provisioned_resources SET state = ?, updated_at = ? WHERE id = ?"),
        )
        .bind(state)
        .bind(asgard_storage::now())
        .bind(id)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn mark_destroyed(&self, id: &str) -> Result<(), ProvisionError> {
        self.mark_state(id, "destroyed").await
    }

    /// Committed recurring infra estimate for one project (provisioned only).
    /// Feeds the auto-approval project-headroom check.
    pub async fn infra_total_for_project(&self, project_id: &str) -> Result<f64, ProvisionError> {
        let total: Option<f64> = sqlx::query_scalar(
            &self
                .db
                .q("SELECT SUM(est_monthly_usd) FROM provisioned_resources \
             WHERE project_id = ? AND state = 'provisioned'"),
        )
        .bind(project_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(total.unwrap_or(0.0))
    }

    /// Recurring infra cost estimate per project (provisioned resources only).
    pub async fn infra_cost_by_project(&self) -> Result<Vec<(String, f64)>, ProvisionError> {
        let rows = sqlx::query(
            "SELECT project_id, SUM(est_monthly_usd) AS total FROM provisioned_resources \
             WHERE state = 'provisioned' GROUP BY project_id ORDER BY total DESC",
        )
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<String, _>("project_id"),
                    r.get::<Option<f64>, _>("total").unwrap_or(0.0),
                )
            })
            .collect())
    }
}

fn row_to_record(r: sqlx::any::AnyRow) -> ProvisionedRecord {
    let spec: String = r.get("spec");
    let outputs: String = r.get("outputs");
    let tags: String = r.get("tags");
    ProvisionedRecord {
        id: r.get("id"),
        project_id: r.get("project_id"),
        rtype: r.get("rtype"),
        name: r.get("name"),
        spec: serde_json::from_str(&spec).unwrap_or(serde_json::Value::Null),
        outputs: serde_json::from_str(&outputs).unwrap_or(serde_json::Value::Null),
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        est_monthly_usd: r.get("est_monthly_usd"),
        state: r.get("state"),
        backend: r.get("backend"),
        dry_run: r.get::<i64, _>("dry_run") != 0,
        request_id: r.get("request_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}
