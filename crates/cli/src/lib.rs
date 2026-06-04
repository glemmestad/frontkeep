//! CLI client logic (brief §4 surfaces, §6.11). Thin HTTP client against the
//! running server plus local helpers (config scaffold, template scaffold, and
//! offline manifest validation). The `asgard` binary owns the clap tree and
//! dispatches into these functions.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use asgard_catalog::{Manifest, SchemaRegistry};

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("http: {0}")]
    Http(String),
    #[error("server returned {status}: {body}")]
    Server { status: u16, body: String },
    #[error("io: {0}")]
    Io(String),
    #[error("yaml: {0}")]
    Yaml(String),
    #[error("validation failed:\n{0}")]
    Invalid(String),
    #[error("unknown template: {0}")]
    UnknownTemplate(String),
}

#[derive(Debug, Deserialize)]
pub struct EntitySummary {
    pub kind: String,
    pub metadata: MetaSummary,
    pub lifecycle: String,
}

#[derive(Debug, Deserialize)]
pub struct MetaSummary {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub title: Option<String>,
}

pub struct Client {
    url: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl Client {
    pub fn new(url: impl Into<String>, token: Option<String>) -> Self {
        Client {
            url: url.into().trim_end_matches('/').to_string(),
            token,
            http: reqwest::Client::new(),
        }
    }

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => rb.bearer_auth(t),
            None => rb,
        }
    }

    pub async fn health(&self) -> Result<bool, CliError> {
        let resp = self
            .http
            .get(format!("{}/healthz", self.url))
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        Ok(resp.status().is_success())
    }

    pub async fn catalog_ls(
        &self,
        kind: Option<&str>,
        q: Option<&str>,
    ) -> Result<Vec<EntitySummary>, CliError> {
        let mut req = self.http.get(format!("{}/api/catalog/entities", self.url));
        if let Some(k) = kind {
            req = req.query(&[("kind", k)]);
        }
        if let Some(query) = q {
            req = req.query(&[("q", query)]);
        }
        let resp = self
            .auth(req)
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }

    /// Mint a per-project virtual key.
    pub async fn gateway_login(&self, project: &str) -> Result<serde_json::Value, CliError> {
        let resp = self
            .auth(
                self.http
                    .post(format!("{}/api/projects/{}/keys", self.url, project))
                    .json(&serde_json::json!({ "name": "cli" })),
            )
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }

    /// Register a project (the mandatory gate). Returns the minted record.
    pub async fn register_project(
        &self,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let resp = self
            .auth(
                self.http
                    .post(format!("{}/api/projects", self.url))
                    .json(&body),
            )
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }

    pub async fn list_projects(&self) -> Result<serde_json::Value, CliError> {
        let resp = self
            .auth(self.http.get(format!("{}/api/projects", self.url)))
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }

    /// Cost rolled up by a dimension (project|owner|manager|group|...).
    pub async fn cost(&self, by: &str) -> Result<serde_json::Value, CliError> {
        let resp = self
            .auth(
                self.http
                    .get(format!("{}/api/cost", self.url))
                    .query(&[("by", by)]),
            )
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }

    /// File a request (request → approval → fulfillment).
    pub async fn submit_request(
        &self,
        kind: &str,
        requester: &str,
        subject: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let resp =
            self.auth(self.http.post(format!("{}/api/requests", self.url)).json(
                &serde_json::json!({
                    "kind": kind,
                    "requester": requester,
                    "subject": subject,
                    "payload": payload,
                }),
            ))
            .send()
            .await
            .map_err(|e| CliError::Http(e.to_string()))?;
        decode(resp).await
    }
}

async fn decode<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, CliError> {
    let status = resp.status();
    if !status.is_success() {
        return Err(CliError::Server {
            status: status.as_u16(),
            body: resp.text().await.unwrap_or_default(),
        });
    }
    resp.json::<T>()
        .await
        .map_err(|e| CliError::Http(e.to_string()))
}

/// Write a starter `asgard.yaml` config into `dir`.
pub fn init_config(dir: &Path) -> Result<PathBuf, CliError> {
    let path = dir.join("asgard.yaml");
    let body = "# Asgard configuration.\n\
                database_url: sqlite://asgard.db\n\
                bind: 0.0.0.0:8080\n\
                # Catalog source repos to reconcile (needs a Git token in ASGARD_GIT_TOKEN).\n\
                sources: []\n\
                #  - provider: github\n\
                #    owner: your-org\n\
                #    repo: your-repo\n\
                #    ref: main\n\
                # Cost-centers a project may register against. Leave empty to accept\n\
                # any group (open mode); list entries to enforce an allowlist.\n\
                groups: []\n\
                #  - key: platform\n\
                #    display_name: Platform Engineering\n\
                #    cost_center: CC-1001\n\
                #  - key: research\n\
                #    display_name: Research\n\
                #    cost_center: CC-2002\n\
                # Provisioning + cost. Credentials come from the environment, never here.\n\
                # provisioning:\n\
                #   rollup_secs: 3600          # cost-rollup cadence (idempotent per day)\n\
                #   forecast_window_days: 60\n\
                #   anomaly_z: 3.0\n\
                #   default_cloud: aws\n\
                #   default_account: \"123456789012\"\n\
                #   allowed:\n\
                #     - { cloud: aws, account: \"123456789012\" }\n\
                #   # Terraform connector: the universal provisioning path. Relative manifest\n\
                #   # `module` paths resolve against modules_dir; the subprocess inherits this\n\
                #   # process's env (AWS_PROFILE/AWS_REGION, AUTH0_* from .env).\n\
                #   terraform:\n\
                #     bin: terraform\n\
                #     modules_dir: /path/to/asgard/modules-root\n\
                #     work_dir: /tmp/asgard-tf\n\
                #   aws:                         # AWS *cost* reads (provisioning is terraform)\n\
                #     region: us-west-2\n\
                #     profile: my-aws-profile\n\
                #     cost_explorer: true      # live, read-only billing (real spend in the dashboard)\n";
    std::fs::write(&path, body).map_err(|e| CliError::Io(e.to_string()))?;
    Ok(path)
}

const TPL_CODE_REVIEW: &[(&str, &str)] = &[
    (
        "agent.yaml",
        include_str!("../../../templates/code-review/agent.yaml"),
    ),
    (
        "prompt.yaml",
        include_str!("../../../templates/code-review/prompt.yaml"),
    ),
    (
        "eval.yaml",
        include_str!("../../../templates/code-review/eval.yaml"),
    ),
    (
        "cases.json",
        include_str!("../../../templates/code-review/cases.json"),
    ),
    (
        "README.md",
        include_str!("../../../templates/code-review/README.md"),
    ),
];

/// Scaffold a golden-path template into `dir`.
pub fn agent_new(template: &str, dir: &Path) -> Result<Vec<PathBuf>, CliError> {
    let files = match template {
        "code-review" => TPL_CODE_REVIEW,
        other => return Err(CliError::UnknownTemplate(other.to_string())),
    };
    std::fs::create_dir_all(dir).map_err(|e| CliError::Io(e.to_string()))?;
    let mut written = Vec::new();
    for (name, content) in files {
        let p = dir.join(name);
        std::fs::write(&p, content).map_err(|e| CliError::Io(e.to_string()))?;
        written.push(p);
    }
    Ok(written)
}

/// Validate a manifest file against its kind's JSON Schema (offline).
pub fn validate_manifest(path: &Path) -> Result<String, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| CliError::Io(e.to_string()))?;
    let registry = SchemaRegistry::embedded().map_err(|e| CliError::Yaml(e.to_string()))?;
    let manifests: Vec<Manifest> =
        asgard_catalog::parse_manifests(&content).map_err(|e| CliError::Yaml(e.to_string()))?;
    for m in &manifests {
        if registry.known_kind(&m.kind) {
            if let Err(errs) = registry.validate(&m.kind, &m.as_value()) {
                return Err(CliError::Invalid(errs.join("\n")));
            }
        }
    }
    Ok(format!("{} manifest(s) valid", manifests.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_and_validates_template() {
        let dir = std::env::temp_dir().join(format!("asgard-cli-{}", asgard_storage::new_uid()));
        let written = agent_new("code-review", &dir).unwrap();
        assert!(written.iter().any(|p| p.ends_with("agent.yaml")));
        // The scaffolded agent manifest validates against the schema.
        let report = validate_manifest(&dir.join("agent.yaml")).unwrap();
        assert!(report.contains("valid"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_template_errors() {
        let dir = std::env::temp_dir().join("asgard-cli-x");
        assert!(matches!(
            agent_new("nope", &dir),
            Err(CliError::UnknownTemplate(_))
        ));
    }

    #[test]
    fn init_writes_config() {
        let dir =
            std::env::temp_dir().join(format!("asgard-cli-init-{}", asgard_storage::new_uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = init_config(&dir).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
