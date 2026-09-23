//! Project members: the collaborator grant. A member operates the project's
//! resources (provision, deploy, secrets, gateway keys) exactly as the owner
//! does, but is not a *steward* — they cannot change the project record, its
//! lifecycle, or its membership, and they never inherit the manager's approval
//! authority (`may_approve_request` stays manager-or-admin).

use frontkeep_storage::Db;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashSet;

use crate::RegistryError;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectMember {
    pub project_id: String,
    pub email: String,
    pub added_by: String,
    pub created_at: String,
}

fn row_to_member(row: &sqlx::any::AnyRow) -> ProjectMember {
    ProjectMember {
        project_id: row.get("project_id"),
        email: row.get("email"),
        added_by: row.get("added_by"),
        created_at: row.get("created_at"),
    }
}

pub async fn list(db: &Db, project_id: &str) -> Result<Vec<ProjectMember>, RegistryError> {
    let sql = db.q(
        "SELECT project_id, email, added_by, created_at FROM project_members \
                    WHERE project_id = ? ORDER BY created_at",
    );
    let rows = sqlx::query(&sql)
        .bind(project_id)
        .fetch_all(db.pool())
        .await?;
    Ok(rows.iter().map(row_to_member).collect())
}

pub async fn is_member(db: &Db, project_id: &str, email: &str) -> Result<bool, RegistryError> {
    let sql = db.q("SELECT 1 FROM project_members WHERE project_id = ? AND email = ?");
    Ok(sqlx::query(&sql)
        .bind(project_id)
        .bind(email)
        .fetch_optional(db.pool())
        .await?
        .is_some())
}

/// Every project `email` is a member of. The in-memory counterpart to the
/// `project_members` subquery the cost queries use.
pub async fn project_ids(db: &Db, email: &str) -> Result<HashSet<String>, RegistryError> {
    let sql = db.q("SELECT project_id FROM project_members WHERE email = ?");
    let rows = sqlx::query(&sql).bind(email).fetch_all(db.pool()).await?;
    Ok(rows.iter().map(|r| r.get("project_id")).collect())
}

pub async fn add(
    db: &Db,
    project_id: &str,
    email: &str,
    added_by: &str,
) -> Result<ProjectMember, RegistryError> {
    let now = frontkeep_storage::now();
    let sql = db.q(
        "INSERT INTO project_members (project_id, email, added_by, created_at) \
                    VALUES (?, ?, ?, ?)",
    );
    sqlx::query(&sql)
        .bind(project_id)
        .bind(email)
        .bind(added_by)
        .bind(&now)
        .execute(db.pool())
        .await?;
    Ok(ProjectMember {
        project_id: project_id.to_string(),
        email: email.to_string(),
        added_by: added_by.to_string(),
        created_at: now,
    })
}

/// Returns whether a row was actually removed, so the caller can 404 a no-op.
pub async fn remove(db: &Db, project_id: &str, email: &str) -> Result<bool, RegistryError> {
    let sql = db.q("DELETE FROM project_members WHERE project_id = ? AND email = ?");
    let res = sqlx::query(&sql)
        .bind(project_id)
        .bind(email)
        .execute(db.pool())
        .await?;
    Ok(res.rows_affected() > 0)
}
