//! Exec connector: the escape hatch for non-IaC and SaaS services. Runs an
//! operator-configured command with the request spec + immutable project tags as
//! JSON on stdin; the command's JSON stdout becomes the resource outputs. Output
//! keys named in the manifest's `secret_outputs` are routed to the secret store
//! by the caller (only refs land in the resource record).

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::{Plan, ProvisionError, ProvisionRequest, Provisioned, Provisioner};

#[derive(Default)]
pub struct ExecConnector;

impl ExecConnector {
    pub fn new() -> Self {
        ExecConnector
    }
}

fn command_of(config: &Value, key: &str) -> Option<Vec<String>> {
    config.get(key).and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect()
    })
}

async fn run(cmd: &[String], stdin_json: &str) -> Result<Value, ProvisionError> {
    let Some((bin, args)) = cmd.split_first() else {
        return Err(ProvisionError::Backend("exec command is empty".into()));
    };
    let mut child = Command::new(bin)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| ProvisionError::Backend(format!("spawn exec connector: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_json.as_bytes())
            .await
            .map_err(|e| ProvisionError::Backend(format!("write exec stdin: {e}")))?;
    }
    let out = child
        .wait_with_output()
        .await
        .map_err(|e| ProvisionError::Backend(format!("exec connector: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ProvisionError::Backend(format!(
            "exec command failed: {}",
            stderr.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(stdout.trim())
        .map_err(|e| ProvisionError::Backend(format!("exec stdout not json: {e}")))
}

#[async_trait]
impl Provisioner for ExecConnector {
    fn name(&self) -> &str {
        "exec"
    }
    fn dry_run(&self) -> bool {
        false
    }
    fn supports(&self, _resource_type: &str) -> bool {
        true
    }

    async fn plan(&self, req: &ProvisionRequest) -> Result<Plan, ProvisionError> {
        Ok(Plan {
            summary: format!("exec connector would run '{}'", req.resource_type),
            tags: req.ctx.tags(),
            estimated_monthly_usd: req.estimated_monthly_usd,
        })
    }

    async fn apply(
        &self,
        req: &ProvisionRequest,
        plan: &Plan,
    ) -> Result<Provisioned, ProvisionError> {
        let cmd = command_of(&req.config, "command")
            .ok_or_else(|| ProvisionError::Backend("exec connector needs config.command".into()))?;
        let stdin = serde_json::json!({
            "name": req.name,
            "spec": req.spec,
            "tags": plan.tags,
        })
        .to_string();
        let outputs = run(&cmd, &stdin).await?;
        Ok(Provisioned {
            outputs,
            resource_ids: vec![],
            sensitive_keys: req.secret_outputs.clone(),
        })
    }

    async fn destroy(
        &self,
        req: &ProvisionRequest,
        _outputs: &Value,
    ) -> Result<(), ProvisionError> {
        if let Some(cmd) = command_of(&req.config, "destroy_command") {
            let stdin = serde_json::json!({ "name": req.name, "spec": req.spec }).to_string();
            run(&cmd, &stdin).await?;
        }
        Ok(())
    }

    async fn stop(&self, req: &ProvisionRequest, _outputs: &Value) -> Result<bool, ProvisionError> {
        let Some(cmd) = command_of(&req.config, "stop_command") else {
            return Ok(false);
        };
        let stdin = serde_json::json!({ "name": req.name, "spec": req.spec }).to_string();
        run(&cmd, &stdin).await?;
        Ok(true)
    }

    async fn resume(
        &self,
        req: &ProvisionRequest,
        _outputs: &Value,
    ) -> Result<bool, ProvisionError> {
        let Some(cmd) = command_of(&req.config, "resume_command") else {
            return Ok(false);
        };
        let stdin = serde_json::json!({ "name": req.name, "spec": req.spec }).to_string();
        run(&cmd, &stdin).await?;
        Ok(true)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::ResourceContext;

    fn ctx() -> ResourceContext {
        ResourceContext {
            project_id: "proj-x".into(),
            owner: "o".into(),
            manager: "m".into(),
            group: "g".into(),
            cost_center: "cc".into(),
            classification: "poc".into(),
            environment: "dev".into(),
            cloud: "stub".into(),
            account: "local".into(),
        }
    }

    fn req(config: Value) -> ProvisionRequest {
        ProvisionRequest {
            resource_type: "exec-svc".into(),
            name: "thing".into(),
            ctx: ctx(),
            spec: serde_json::json!({"name": "thing"}),
            config,
            estimated_monthly_usd: 1.0,
            secret_outputs: vec![],
        }
    }

    #[tokio::test]
    async fn apply_runs_command_and_parses_json_stdout() {
        let c = ExecConnector::new();
        let r = req(serde_json::json!({
            "command": ["sh", "-c", "cat >/dev/null; echo '{\"ok\": true, \"id\": \"abc\"}'"]
        }));
        let plan = c.plan(&r).await.unwrap();
        let out = c.apply(&r, &plan).await.unwrap();
        assert_eq!(out.outputs["ok"], serde_json::json!(true));
        assert_eq!(out.outputs["id"], serde_json::json!("abc"));
    }

    #[tokio::test]
    async fn apply_without_command_errors() {
        let c = ExecConnector::new();
        let r = req(serde_json::json!({}));
        let plan = c.plan(&r).await.unwrap();
        assert!(c.apply(&r, &plan).await.is_err());
    }

    #[tokio::test]
    async fn destroy_is_noop_without_destroy_command() {
        let c = ExecConnector::new();
        let r = req(serde_json::json!({"command": ["true"]}));
        assert!(c.destroy(&r, &Value::Null).await.is_ok());
    }
}
