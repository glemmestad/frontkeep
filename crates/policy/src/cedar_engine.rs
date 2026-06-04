use std::str::FromStr;

use async_trait::async_trait;
use cedar_policy::{
    Authorizer, Context, Decision as CedarDecision, Entities, EntityUid, PolicySet,
    Request as CedarRequest,
};

use crate::{Decision, Effect, Obligation, PolicyEngine, PolicyError, Request};

const DEFAULT_POLICIES: &str = include_str!("../policies/default.cedar");

/// In-process Cedar policy engine. Decision facts are carried on the request
/// context (see `policies/default.cedar`), so no entity store is required.
pub struct CedarEngine {
    policies: PolicySet,
    authorizer: Authorizer,
}

impl CedarEngine {
    /// Build from the bundled default policies.
    pub fn new() -> Result<CedarEngine, PolicyError> {
        CedarEngine::from_policies(DEFAULT_POLICIES)
    }

    pub fn from_policies(text: &str) -> Result<CedarEngine, PolicyError> {
        let policies = PolicySet::from_str(text).map_err(|e| PolicyError::Parse(e.to_string()))?;
        Ok(CedarEngine {
            policies,
            authorizer: Authorizer::new(),
        })
    }

    fn evaluate(&self, req: &Request) -> Result<Decision, PolicyError> {
        let principal = to_uid(&req.principal)?;
        let action = action_uid(&req.action)?;
        let resource = to_uid(&req.resource)?;
        let context = Context::from_json_value(req.context.clone(), None)
            .map_err(|e| PolicyError::Context(e.to_string()))?;
        let request = CedarRequest::new(principal, action, resource, context, None)
            .map_err(|e| PolicyError::Request(e.to_string()))?;

        let response = self
            .authorizer
            .is_authorized(&request, &self.policies, &Entities::empty());

        let effect = match response.decision() {
            CedarDecision::Allow => Effect::Allow,
            CedarDecision::Deny => Effect::Deny,
        };

        let mut reasons = Vec::new();
        let mut obligations = Vec::new();
        for pid in response.diagnostics().reason() {
            reasons.push(pid.to_string());
            if let Some(policy) = self.policies.policy(pid) {
                if let Some(approver) = policy.annotation("approval") {
                    obligations.push(Obligation::RequiresApproval {
                        approver: approver.to_string(),
                    });
                }
            }
        }
        if reasons.is_empty() {
            reasons.push(match effect {
                Effect::Allow => "allowed by default".into(),
                Effect::Deny => "no matching permit (default deny)".into(),
            });
        }

        Ok(Decision {
            effect,
            reasons,
            obligations,
        })
    }
}

#[async_trait]
impl PolicyEngine for CedarEngine {
    async fn is_authorized(&self, req: &Request) -> Decision {
        self.evaluate(req)
            .unwrap_or_else(|e| Decision::deny(format!("policy error: {e}")))
    }
}

fn to_uid(reference: &str) -> Result<EntityUid, PolicyError> {
    let (kind, id) = match reference.split_once(':') {
        Some((k, rest)) => (k.to_string(), rest.to_string()),
        None => ("entity".to_string(), reference.to_string()),
    };
    let ty = cedar_type(&kind);
    let s = format!("{ty}::\"{id}\"");
    EntityUid::from_str(&s).map_err(|e| PolicyError::Parse(format!("uid '{s}': {e}")))
}

fn action_uid(action: &str) -> Result<EntityUid, PolicyError> {
    let s = format!("Action::\"{action}\"");
    EntityUid::from_str(&s).map_err(|e| PolicyError::Parse(format!("action '{s}': {e}")))
}

fn cedar_type(kind: &str) -> String {
    match kind {
        "user" => "User".to_string(),
        "group" => "Group".to_string(),
        "agent" => "Agent".to_string(),
        "model" => "Model".to_string(),
        "project" => "Project".to_string(),
        "dataset" => "Dataset".to_string(),
        "tool" | "mcpserver" => "Tool".to_string(),
        "prompt" => "Prompt".to_string(),
        "eval" => "Eval".to_string(),
        other => capitalize(other),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Entity".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn invoke_allowed_when_data_class_on_allowlist() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "invoke",
                "model:default/gpt-4o",
                json!({"data_class": "internal", "model_data_classes": ["public", "internal"]}),
            ))
            .await;
        assert!(d.allowed(), "reasons: {:?}", d.reasons);
    }

    #[tokio::test]
    async fn invoke_denied_when_data_class_not_allowed() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "invoke",
                "model:default/gpt-4o",
                json!({"data_class": "restricted", "model_data_classes": ["public", "internal"]}),
            ))
            .await;
        assert!(!d.allowed(), "should deny wrong data-class/model pairing");
    }

    #[tokio::test]
    async fn restricted_deploy_requires_approval() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "deploy",
                "agent:default/triage",
                json!({"data_class": "restricted"}),
            ))
            .await;
        assert!(d.allowed());
        assert_eq!(d.requires_approval(), Some("group:default/security"));
    }

    #[tokio::test]
    async fn promote_to_light_auto_approves_when_clean() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "promote",
                "project:default/x",
                json!({"target_classification": "light-operational", "evidence_complete": true, "has_exception": false}),
            ))
            .await;
        assert!(d.allowed());
        assert_eq!(
            d.requires_approval(),
            None,
            "clean Light is machine-approved"
        );
    }

    #[tokio::test]
    async fn promote_to_light_with_exception_routes_to_platform() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "promote",
                "project:default/x",
                json!({"target_classification": "light-operational", "evidence_complete": true, "has_exception": true}),
            ))
            .await;
        assert!(d.allowed());
        assert_eq!(d.requires_approval(), Some("group:default/platform"));
    }

    #[tokio::test]
    async fn promote_to_wide_always_routes_to_platform() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "promote",
                "project:default/x",
                json!({"target_classification": "wide-operational", "evidence_complete": true, "has_exception": false}),
            ))
            .await;
        assert!(d.allowed());
        assert_eq!(d.requires_approval(), Some("group:default/platform"));
    }

    #[tokio::test]
    async fn promote_to_critical_requires_security_and_risk_acceptance() {
        let eng = CedarEngine::new().unwrap();
        // Without recorded risk acceptance, the forbid overrides the security permit.
        let denied = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "promote",
                "project:default/x",
                json!({"target_classification": "critical-path", "evidence_complete": true, "has_exception": false, "risk_accepted": false}),
            ))
            .await;
        assert!(
            !denied.allowed(),
            "no risk acceptance must forbid critical-path"
        );
        // With risk acceptance, it's permitted but still needs security approval.
        let routed = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "promote",
                "project:default/x",
                json!({"target_classification": "critical-path", "evidence_complete": true, "has_exception": false, "risk_accepted": true}),
            ))
            .await;
        assert!(routed.allowed());
        assert_eq!(routed.requires_approval(), Some("group:default/security"));
    }

    #[tokio::test]
    async fn killed_project_forbids_everything() {
        let eng = CedarEngine::new().unwrap();
        let d = eng
            .is_authorized(&Request::new(
                "user:default/alice",
                "read",
                "project:default/x",
                json!({"project_killed": true}),
            ))
            .await;
        assert!(!d.allowed(), "kill switch must override permits");
    }
}
