//! Governance / portfolio metrics (WS4). The policy's operating cadence runs on a
//! Metrics table — owner-less systems, support-path coverage, stale POCs,
//! two-maintainer coverage, promotion cycle time, stale inventory, unsupported
//! stacks. These are read-only queries over the registry (WS1 evidence + WS2
//! promotion timestamps + WS3 review state); nothing new in the data model.
//!
//! `compute` is pure over an already-scoped project list + promotion samples, so
//! every metric definition is unit-testable with no DB. Two honesty rules from
//! the spec: metrics with no backing data source are *labelled* (`measurable:
//! false` + a note), never silently reported as 0; and the one org-specific
//! threshold (`maintainer_min`) lives in config, not a frozen const.

use crate::Registration;
use serde::Serialize;

/// The one org-specific governance threshold. Default mirrors the policy doc's
/// two-maintainer rule.
#[derive(Debug, Clone)]
pub struct GovernanceConfig {
    pub maintainer_min: i64,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        GovernanceConfig { maintainer_min: 2 }
    }
}

/// One portfolio metric. `value` is `None` for an unmeasurable stub; `offenders`
/// carries the project ids behind a count so the UI can drill in.
#[derive(Debug, Clone, Serialize)]
pub struct Metric {
    pub key: String,
    pub label: String,
    pub target: String,
    pub value: Option<f64>,
    pub unit: String,
    pub offenders: Vec<String>,
    pub measurable: bool,
    pub note: Option<String>,
}

/// The full portfolio snapshot returned to REST/MCP/UI.
#[derive(Debug, Clone, Serialize)]
pub struct GovernanceMetrics {
    pub as_of: String,
    pub total_projects: i64,
    pub operational_projects: i64,
    pub metrics: Vec<Metric>,
}

/// One fulfilled promotion, for cycle-time. `seconds` is request→fulfilled.
#[derive(Debug, Clone, Serialize)]
pub struct PromotionSample {
    pub project_id: String,
    pub target: String,
    pub seconds: i64,
}

fn is_active(r: &Registration) -> bool {
    r.registered && r.lifecycle == "active"
}

/// "Operational" per the spec: registered, active, and past the POC stage.
fn is_operational(r: &Registration) -> bool {
    is_active(r) && r.classification != "poc"
}

/// A measurable count metric: offenders are the projects that trip the predicate.
fn count_metric(
    key: &str,
    label: &str,
    target: &str,
    offenders: Vec<String>,
    note: Option<String>,
) -> Metric {
    Metric {
        key: key.into(),
        label: label.into(),
        target: target.into(),
        value: Some(offenders.len() as f64),
        unit: "count".into(),
        offenders,
        measurable: true,
        note,
    }
}

/// A stub for a metric with no backing data source yet — explicitly labelled, so
/// it reads as "unmeasured", never as a real zero.
fn stub_metric(key: &str, label: &str, target: &str, note: &str) -> Metric {
    Metric {
        key: key.into(),
        label: label.into(),
        target: target.into(),
        value: None,
        unit: "count".into(),
        offenders: Vec::new(),
        measurable: false,
        note: Some(note.into()),
    }
}

fn ids(projects: &[&Registration]) -> Vec<String> {
    projects.iter().map(|r| r.project_id.clone()).collect()
}

/// Compute the portfolio metrics over an already-scoped project list. Pure.
pub fn compute(
    projects: &[Registration],
    samples: &[PromotionSample],
    cfg: &GovernanceConfig,
    as_of: &str,
) -> GovernanceMetrics {
    let operational: Vec<&Registration> = projects.iter().filter(|r| is_operational(r)).collect();
    let active: Vec<&Registration> = projects.iter().filter(|r| is_active(r)).collect();

    let ownerless: Vec<&Registration> = operational
        .iter()
        .copied()
        .filter(|r| r.owner.trim().is_empty())
        .collect();
    let no_support: Vec<&Registration> = operational
        .iter()
        .copied()
        .filter(|r| r.evidence.support_contact.trim().is_empty())
        .collect();
    let understaffed: Vec<&Registration> = operational
        .iter()
        .copied()
        .filter(|r| {
            matches!(
                r.classification.as_str(),
                "wide-operational" | "critical-path"
            ) && (r.evidence.maintainers.len() as i64) < cfg.maintainer_min
        })
        .collect();
    let stale_pocs: Vec<&Registration> = active
        .iter()
        .copied()
        .filter(|r| r.classification == "poc" && r.review_state == "expired")
        .collect();
    let stale_inventory: Vec<&Registration> = operational
        .iter()
        .copied()
        .filter(|r| r.review_state == "expired")
        .collect();
    let unsupported_stack: Vec<&Registration> = active
        .iter()
        .copied()
        .filter(|r| !r.evidence.stack_exception.trim().is_empty())
        .collect();

    let no_renewal = unsupported_stack
        .iter()
        .filter(|r| {
            let d = r.stack_exception_renewal_date.trim();
            d.is_empty() || d < as_of
        })
        .count();
    let stack_note = format!(
        "{} of {} have no future renewal date (lapsed/unbounded exception)",
        no_renewal,
        unsupported_stack.len()
    );

    let mut metrics = vec![
        count_metric(
            "ownerless_operational",
            "Operational systems without a named owner",
            "0",
            ids(&ownerless),
            Some("≈0 until WS5 ingests owner-less shadow systems".into()),
        ),
        count_metric(
            "no_support_path_operational",
            "Operational systems without a support path",
            "0",
            ids(&no_support),
            None,
        ),
        count_metric(
            "understaffed_wide_critical",
            &format!(
                "Wide/Critical systems with fewer than {} maintainers",
                cfg.maintainer_min
            ),
            "0",
            ids(&understaffed),
            None,
        ),
        count_metric(
            "stale_pocs",
            "POCs past their review date without a decision",
            "down",
            ids(&stale_pocs),
            None,
        ),
        count_metric(
            "stale_inventory",
            "Operational systems with an expired review",
            "down",
            ids(&stale_inventory),
            None,
        ),
        count_metric(
            "unsupported_stack",
            "Systems running on an unsupported stack (exception declared)",
            "down/justified",
            ids(&unsupported_stack),
            Some(stack_note),
        ),
        light_cycle_metric(samples),
        stub_metric(
            "missing_inventory_record",
            "Shadow/unregistered operational systems",
            "0",
            "no data source yet — needs WS5 existing-system discovery",
        ),
        stub_metric(
            "incidents_by_classification",
            "Incidents by classification",
            "—",
            "no data source yet — no incident feed wired up",
        ),
        stub_metric(
            "change_failure_rate",
            "Change failure rate",
            "—",
            "no data source yet — no incident/deploy feed wired up",
        ),
    ];
    metrics.shrink_to_fit();

    GovernanceMetrics {
        as_of: as_of.to_string(),
        total_projects: projects.len() as i64,
        operational_projects: operational.len() as i64,
        metrics,
    }
}

/// Mean days from promotion request to fulfilment for Light-operational targets.
/// Measurable only with ≥1 fulfilled Light promotion; otherwise an honest stub.
fn light_cycle_metric(samples: &[PromotionSample]) -> Metric {
    let light: Vec<&PromotionSample> = samples
        .iter()
        .filter(|s| s.target == "light-operational")
        .collect();
    if light.is_empty() {
        return Metric {
            key: "light_promotion_cycle_days".into(),
            label: "Light-operational promotion cycle time".into(),
            target: "short".into(),
            value: None,
            unit: "days".into(),
            offenders: Vec::new(),
            measurable: false,
            note: Some("no fulfilled Light-operational promotions yet".into()),
        };
    }
    let mean_secs = light.iter().map(|s| s.seconds).sum::<i64>() as f64 / light.len() as f64;
    Metric {
        key: "light_promotion_cycle_days".into(),
        label: "Light-operational promotion cycle time".into(),
        target: "short".into(),
        value: Some(mean_secs / 86_400.0),
        unit: "days".into(),
        offenders: light.iter().map(|s| s.project_id.clone()).collect(),
        measurable: true,
        note: Some(format!("mean over {} promotion(s)", light.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Evidence;

    fn base() -> Registration {
        Registration {
            project_id: "proj-2026-0001".into(),
            name: "X".into(),
            owner: "owner@corp.example".into(),
            manager: "mgr@corp.example".into(),
            group: "platform".into(),
            cost_center: "CC-100".into(),
            classification: "light-operational".into(),
            data_class: "internal".into(),
            budget_usd: 0.0,
            spent_usd: 0.0,
            lifecycle: "active".into(),
            registered: true,
            killed: false,
            description: String::new(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
            review_date: String::new(),
            review_state: "ok".into(),
            review_extensions: 0,
            stack_exception_renewal_date: String::new(),
            evidence: Evidence::default(),
        }
    }

    fn metric<'a>(m: &'a GovernanceMetrics, key: &str) -> &'a Metric {
        m.metrics.iter().find(|x| x.key == key).unwrap()
    }

    const NOW: &str = "2026-06-03T00:00:00.000Z";

    #[test]
    fn ownerless_only_counts_operational() {
        let mut poc = base();
        poc.classification = "poc".into();
        poc.owner = String::new();
        let mut op = base();
        op.project_id = "proj-2026-0002".into();
        op.owner = String::new();
        let m = compute(&[poc, op], &[], &GovernanceConfig::default(), NOW);
        let ownerless = metric(&m, "ownerless_operational");
        assert_eq!(ownerless.value, Some(1.0));
        assert_eq!(ownerless.offenders, vec!["proj-2026-0002"]);
        assert_eq!(m.operational_projects, 1);
        assert_eq!(m.total_projects, 2);
    }

    #[test]
    fn no_support_path_flags_operational_without_contact() {
        let with = {
            let mut r = base();
            r.evidence.support_contact = "oncall@corp.example".into();
            r
        };
        let without = {
            let mut r = base();
            r.project_id = "proj-2026-0002".into();
            r
        };
        let m = compute(&[with, without], &[], &GovernanceConfig::default(), NOW);
        let metric = metric(&m, "no_support_path_operational");
        assert_eq!(metric.offenders, vec!["proj-2026-0002"]);
    }

    #[test]
    fn understaffed_respects_threshold_boundary() {
        let mut wide = base();
        wide.classification = "wide-operational".into();
        wide.evidence.maintainers = vec!["a@corp.example".into()]; // 1 < 2 → trips
        let mut wide_ok = base();
        wide_ok.project_id = "proj-2026-0002".into();
        wide_ok.classification = "wide-operational".into();
        wide_ok.evidence.maintainers = vec!["a@corp.example".into(), "b@corp.example".into()]; // 2 == min → ok
        let m = compute(&[wide, wide_ok], &[], &GovernanceConfig::default(), NOW);
        let understaffed = metric(&m, "understaffed_wide_critical");
        assert_eq!(understaffed.offenders, vec!["proj-2026-0001"]);
        // A lower threshold clears the boundary case.
        let m2 = compute(&[base()], &[], &GovernanceConfig { maintainer_min: 1 }, NOW);
        // base() is light-operational, never in scope regardless of threshold.
        assert_eq!(metric(&m2, "understaffed_wide_critical").value, Some(0.0));
    }

    #[test]
    fn stale_pocs_need_expired_review() {
        let mut poc = base();
        poc.classification = "poc".into();
        poc.review_state = "expired".into();
        let m = compute(&[poc], &[], &GovernanceConfig::default(), NOW);
        assert_eq!(metric(&m, "stale_pocs").value, Some(1.0));
        // POCs are not "operational" — they don't show in stale_inventory.
        assert_eq!(metric(&m, "stale_inventory").value, Some(0.0));
    }

    #[test]
    fn unsupported_stack_notes_missing_renewal() {
        let mut lapsed = base();
        lapsed.evidence.stack_exception = "legacy runtime".into();
        let mut renewed = base();
        renewed.project_id = "proj-2026-0002".into();
        renewed.evidence.stack_exception = "legacy runtime".into();
        renewed.stack_exception_renewal_date = "2027-01-01T00:00:00.000Z".into();
        let m = compute(&[lapsed, renewed], &[], &GovernanceConfig::default(), NOW);
        let metric = metric(&m, "unsupported_stack");
        assert_eq!(metric.value, Some(2.0));
        assert!(metric.note.as_deref().unwrap().starts_with("1 of 2"));
    }

    #[test]
    fn cycle_time_means_over_light_samples() {
        let samples = vec![
            PromotionSample {
                project_id: "proj-2026-0001".into(),
                target: "light-operational".into(),
                seconds: 2 * 86_400,
            },
            PromotionSample {
                project_id: "proj-2026-0002".into(),
                target: "light-operational".into(),
                seconds: 4 * 86_400,
            },
            PromotionSample {
                project_id: "proj-2026-0003".into(),
                target: "wide-operational".into(),
                seconds: 99 * 86_400,
            },
        ];
        let m = compute(&[], &samples, &GovernanceConfig::default(), NOW);
        let cycle = metric(&m, "light_promotion_cycle_days");
        assert!(cycle.measurable);
        assert_eq!(cycle.value, Some(3.0));
        assert_eq!(cycle.offenders.len(), 2);
    }

    #[test]
    fn cycle_time_is_honest_stub_without_samples() {
        let m = compute(&[], &[], &GovernanceConfig::default(), NOW);
        let cycle = metric(&m, "light_promotion_cycle_days");
        assert!(!cycle.measurable);
        assert!(cycle.value.is_none());
    }

    #[test]
    fn stubs_are_labelled_not_zero() {
        let m = compute(&[], &[], &GovernanceConfig::default(), NOW);
        for key in [
            "missing_inventory_record",
            "incidents_by_classification",
            "change_failure_rate",
        ] {
            let s = metric(&m, key);
            assert!(!s.measurable, "{key} should be a stub");
            assert!(s.value.is_none(), "{key} must not report 0");
            assert!(s.note.is_some());
        }
    }
}
