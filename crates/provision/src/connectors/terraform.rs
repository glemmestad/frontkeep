//! Terraform connector — the universal, unrestricted provisioning path. The
//! "how" lives in hub-supplied TF modules, so any AWS/GCP/Azure (or other)
//! resource Terraform can express is reachable with zero Asgard code per
//! service. The connector materializes a per-resource working dir, writes tfvars
//! from the request spec, injects the immutable project tags as a `tags`
//! variable, runs `init`/`apply`, and captures `terraform output -json`.
//! Sensitive outputs are reported as `sensitive_keys` so the caller routes their
//! values to the secret store — they never land in the resource record.
//!
//! State backend is operator-configurable; the zero-config default is local
//! state persisted under `work_root/<project>/<name>` (kept off the control
//! plane, not stored in the DB record). `destroy` re-uses that working dir.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{Map, Value};
use tokio::process::Command;

use crate::{Plan, ProvisionError, ProvisionRequest, Provisioned, Provisioner};

pub struct TerraformConnector {
    bin: String,
    modules_dir: PathBuf,
    work_root: PathBuf,
}

impl TerraformConnector {
    pub fn new(bin: impl Into<String>, modules_dir: PathBuf, work_root: PathBuf) -> Self {
        TerraformConnector {
            bin: bin.into(),
            modules_dir,
            work_root,
        }
    }

    fn module_path(&self, config: &Value) -> Result<PathBuf, ProvisionError> {
        let m = config
            .get("module")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProvisionError::Backend("terraform connector needs config.module".into())
            })?;
        let p = Path::new(m);
        Ok(if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.modules_dir.join(m)
        })
    }

    fn work_dir(&self, req: &ProvisionRequest) -> PathBuf {
        self.work_root
            .join(&req.ctx.project_id)
            .join(&req.resource_type)
            .join(&req.name)
    }

    /// Materialize the working dir and `terraform apply` the module with the spec
    /// tfvars layered with `overrides` (empty for a normal apply; the suspend vars
    /// for a stop). Captures `output -json`. The same path provisions, suspends,
    /// and resumes — only the override layer differs.
    async fn apply_with(
        &self,
        req: &ProvisionRequest,
        plan: &Plan,
        overrides: &Value,
    ) -> Result<Provisioned, ProvisionError> {
        let module = self.module_path(&req.config)?;
        let wd = self.work_dir(req);
        copy_module(&module, &wd)?;
        let vars = tfvars(req, plan, overrides);
        std::fs::write(
            wd.join("asgard.auto.tfvars.json"),
            serde_json::to_vec_pretty(&vars).unwrap_or_default(),
        )
        .map_err(|e| ProvisionError::Backend(format!("write tfvars: {e}")))?;

        self.run(&wd, &["init", "-input=false", "-no-color"])
            .await?;
        self.run(
            &wd,
            &["apply", "-auto-approve", "-input=false", "-no-color"],
        )
        .await?;
        let out = self.run(&wd, &["output", "-json", "-no-color"]).await?;
        let raw: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);

        let mut outputs = Map::new();
        let mut sensitive_keys = Vec::new();
        if let Some(obj) = raw.as_object() {
            for (k, decl) in obj {
                let value = decl.get("value").cloned().unwrap_or(Value::Null);
                if decl.get("sensitive").and_then(|s| s.as_bool()) == Some(true) {
                    sensitive_keys.push(k.clone());
                }
                outputs.insert(k.clone(), value);
            }
        }
        Ok(Provisioned {
            outputs: Value::Object(outputs),
            resource_ids: vec![],
            sensitive_keys,
        })
    }

    async fn run(&self, dir: &Path, args: &[&str]) -> Result<std::process::Output, ProvisionError> {
        let chdir = format!("-chdir={}", dir.display());
        let mut full = vec![chdir.as_str()];
        full.extend_from_slice(args);
        let out = Command::new(&self.bin)
            .args(&full)
            .output()
            .await
            .map_err(|e| ProvisionError::Backend(format!("spawn terraform: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(ProvisionError::Backend(format!(
                "terraform {} failed: {}",
                args.first().copied().unwrap_or(""),
                stderr.trim()
            )));
        }
        Ok(out)
    }
}

/// Copy module files into the working dir (skipping persisted state + the
/// `.terraform` cache) so re-apply refreshes the module without clobbering state.
fn copy_module(src: &Path, dst: &Path) -> Result<(), ProvisionError> {
    std::fs::create_dir_all(dst)
        .map_err(|e| ProvisionError::Backend(format!("mkdir {}: {e}", dst.display())))?;
    let entries = std::fs::read_dir(src)
        .map_err(|e| ProvisionError::Backend(format!("read module {}: {e}", src.display())))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".terraform" || name_str.starts_with("terraform.tfstate") {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_module(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| ProvisionError::Backend(format!("copy {}: {e}", from.display())))?;
        }
    }
    Ok(())
}

/// Build the tfvars: every top-level spec field, plus `name`, the immutable
/// project `tags` map, and any `overrides` (a suspend/resume re-apply layers
/// these over the spec — e.g. `desired_count: 0`).
fn tfvars(req: &ProvisionRequest, plan: &Plan, overrides: &Value) -> Value {
    let mut m = match &req.spec {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    };
    m.insert("name".into(), Value::String(req.name.clone()));
    let tags: Map<String, Value> = plan
        .tags
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    m.insert("tags".into(), Value::Object(tags));
    if let Value::Object(o) = overrides {
        for (k, v) in o {
            m.insert(k.clone(), v.clone());
        }
    }
    Value::Object(m)
}

#[async_trait]
impl Provisioner for TerraformConnector {
    fn name(&self) -> &str {
        "terraform"
    }
    fn dry_run(&self) -> bool {
        false
    }
    fn supports(&self, _resource_type: &str) -> bool {
        true
    }

    async fn plan(&self, req: &ProvisionRequest) -> Result<Plan, ProvisionError> {
        let module = self.module_path(&req.config)?;
        Ok(Plan {
            summary: format!(
                "terraform apply {} for {}/{}",
                module.display(),
                req.ctx.project_id,
                req.name
            ),
            tags: req.ctx.tags(),
            estimated_monthly_usd: req.estimated_monthly_usd,
        })
    }

    async fn apply(
        &self,
        req: &ProvisionRequest,
        plan: &Plan,
    ) -> Result<Provisioned, ProvisionError> {
        self.apply_with(req, plan, &Value::Null).await
    }

    async fn destroy(
        &self,
        req: &ProvisionRequest,
        _outputs: &Value,
    ) -> Result<(), ProvisionError> {
        let wd = self.work_dir(req);
        if !wd.exists() {
            return Ok(());
        }
        self.run(
            &wd,
            &["destroy", "-auto-approve", "-input=false", "-no-color"],
        )
        .await?;
        Ok(())
    }

    /// Suspend by re-applying the module with `config.stop_tfvars` layered over
    /// the spec (e.g. `desired_count: 0`). A service with no `stop_tfvars` has no
    /// meaningful stop → `false`.
    async fn stop(&self, req: &ProvisionRequest, _outputs: &Value) -> Result<bool, ProvisionError> {
        let Some(overrides) = req.config.get("stop_tfvars") else {
            return Ok(false);
        };
        let plan = self.plan(req).await?;
        self.apply_with(req, &plan, overrides).await?;
        Ok(true)
    }

    /// Resume by re-applying the module with the original spec (no overrides),
    /// restoring the pre-stop state. Only meaningful for a stoppable service.
    async fn resume(
        &self,
        req: &ProvisionRequest,
        _outputs: &Value,
    ) -> Result<bool, ProvisionError> {
        if req.config.get("stop_tfvars").is_none() {
            return Ok(false);
        }
        let plan = self.plan(req).await?;
        self.apply_with(req, &plan, &Value::Null).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceContext;
    use std::collections::BTreeMap;

    fn req() -> ProvisionRequest {
        ProvisionRequest {
            resource_type: "ecs-service".into(),
            name: "web".into(),
            ctx: ResourceContext {
                project_id: "proj-x".into(),
                owner: "o".into(),
                manager: "m".into(),
                group: "g".into(),
                cost_center: "cc".into(),
                classification: "poc".into(),
                environment: "dev".into(),
                cloud: "aws".into(),
                account: "123".into(),
            },
            spec: serde_json::json!({"name": "web", "desired_count": 2}),
            config: serde_json::json!({"stop_tfvars": {"desired_count": 0}}),
            estimated_monthly_usd: 35.0,
            secret_outputs: vec![],
        }
    }

    fn plan() -> Plan {
        Plan {
            summary: String::new(),
            tags: BTreeMap::new(),
            estimated_monthly_usd: 35.0,
        }
    }

    #[test]
    fn stop_tfvars_override_the_spec() {
        let r = req();
        let suspend = tfvars(&r, &plan(), &r.config["stop_tfvars"]);
        assert_eq!(suspend["desired_count"], serde_json::json!(0));
        // A normal apply leaves the spec's value intact.
        let normal = tfvars(&r, &plan(), &Value::Null);
        assert_eq!(normal["desired_count"], serde_json::json!(2));
    }
}
