//! Append-only audit log. Cross-cutting: gateway, policy, workflow, catalog, and
//! identity all write here with the originating trace id (RFC-0001 §5).

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{new_uid, now, Db, StorageError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub ts: String,
    pub actor: String,
    pub action: String,
    pub entity_ref: Option<String>,
    pub trace_id: Option<String>,
    pub outcome: String,
    pub reason: Option<String>,
    /// Arbitrary structured context, stored as JSON text.
    pub data: serde_json::Value,
}

impl AuditRecord {
    pub fn new(actor: impl Into<String>, action: impl Into<String>) -> Self {
        AuditRecord {
            id: new_uid(),
            ts: now(),
            actor: actor.into(),
            action: action.into(),
            entity_ref: None,
            trace_id: None,
            outcome: "ok".into(),
            reason: None,
            data: serde_json::Value::Object(Default::default()),
        }
    }

    pub fn entity(mut self, r: impl Into<String>) -> Self {
        self.entity_ref = Some(r.into());
        self
    }
    pub fn trace(mut self, t: impl Into<String>) -> Self {
        self.trace_id = Some(t.into());
        self
    }
    pub fn outcome(mut self, o: impl Into<String>) -> Self {
        self.outcome = o.into();
        self
    }
    pub fn reason(mut self, r: impl Into<String>) -> Self {
        self.reason = Some(r.into());
        self
    }
    pub fn data(mut self, d: serde_json::Value) -> Self {
        self.data = d;
        self
    }
}

pub async fn append(db: &Db, rec: &AuditRecord) -> Result<(), StorageError> {
    sqlx::query(&db.q(
        "INSERT INTO audit_log (id, ts, actor, action, entity_ref, trace_id, outcome, reason, data)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    ))
    .bind(&rec.id)
    .bind(&rec.ts)
    .bind(&rec.actor)
    .bind(&rec.action)
    .bind(&rec.entity_ref)
    .bind(&rec.trace_id)
    .bind(&rec.outcome)
    .bind(&rec.reason)
    .bind(rec.data.to_string())
    .execute(db.pool())
    .await?;
    Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct AuditQuery {
    pub entity_ref: Option<String>,
    pub trace_id: Option<String>,
    pub actor: Option<String>,
    pub limit: Option<i64>,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn entity(mut self, r: impl Into<String>) -> Self {
        self.entity_ref = Some(r.into());
        self
    }
    pub fn trace(mut self, t: impl Into<String>) -> Self {
        self.trace_id = Some(t.into());
        self
    }
    pub fn actor(mut self, a: impl Into<String>) -> Self {
        self.actor = Some(a.into());
        self
    }
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }
}

pub async fn query(db: &Db, q: &AuditQuery) -> Result<Vec<AuditRecord>, StorageError> {
    let mut sql = String::from(
        "SELECT id, ts, actor, action, entity_ref, trace_id, outcome, reason, data \
         FROM audit_log WHERE 1=1",
    );
    if q.entity_ref.is_some() {
        sql.push_str(" AND entity_ref = ?");
    }
    if q.trace_id.is_some() {
        sql.push_str(" AND trace_id = ?");
    }
    if q.actor.is_some() {
        sql.push_str(" AND actor = ?");
    }
    sql.push_str(" ORDER BY ts DESC");
    let limit = q.limit.unwrap_or(500);
    sql.push_str(&format!(" LIMIT {}", limit));

    let sql = db.q(&sql);
    let mut query = sqlx::query(&sql);
    if let Some(v) = &q.entity_ref {
        query = query.bind(v);
    }
    if let Some(v) = &q.trace_id {
        query = query.bind(v);
    }
    if let Some(v) = &q.actor {
        query = query.bind(v);
    }

    let rows = query.fetch_all(db.pool()).await?;
    rows.into_iter().map(row_to_record).collect()
}

fn row_to_record(row: sqlx::any::AnyRow) -> Result<AuditRecord, StorageError> {
    let data_str: String = row.try_get("data")?;
    Ok(AuditRecord {
        id: row.try_get("id")?,
        ts: row.try_get("ts")?,
        actor: row.try_get("actor")?,
        action: row.try_get("action")?,
        entity_ref: row.try_get("entity_ref")?,
        trace_id: row.try_get("trace_id")?,
        outcome: row.try_get("outcome")?,
        reason: row.try_get("reason")?,
        data: serde_json::from_str(&data_str).unwrap_or(serde_json::Value::Null),
    })
}
