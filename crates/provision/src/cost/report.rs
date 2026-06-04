//! Dashboard aggregation: turns the denormalized `cost_rollup` rows (via
//! [`ProjectFact`]) into the org tree, top movers, and the tagged-% denominator
//! the cost dashboard renders. Pure assembly over data the repo already returns —
//! no read-time join to `projects_runtime`, no I/O of its own.

use std::collections::BTreeMap;

use serde::Serialize;

use super::rollup::ProjectFact;

/// Extra per-project facts the rollup rows don't carry: the budget (from the
/// registration) and the latest end-of-month forecast.
#[derive(Debug, Clone, Default)]
pub struct ProjectOverlay {
    pub budget_usd: f64,
    pub eom_usd: f64,
    pub forecast_band: f64,
    pub has_forecast: bool,
}

/// A node in the org-cost tree: company → group → manager → owner → project.
#[derive(Debug, Clone, Serialize)]
pub struct CostNode {
    pub key: String,
    pub level: String,
    pub mtd_usd: f64,
    pub eom_forecast_usd: f64,
    pub forecast_band: f64,
    pub budget_usd: f64,
    pub budget_pressure: bool,
    pub children: Vec<CostNode>,
}

impl CostNode {
    fn leaf(key: String, level: &str, fact: &ProjectFact, ov: &ProjectOverlay) -> Self {
        let eom = if ov.has_forecast { ov.eom_usd } else { 0.0 };
        CostNode {
            key,
            level: level.into(),
            mtd_usd: round2(fact.mtd_usd),
            eom_forecast_usd: round2(eom),
            forecast_band: round2(ov.forecast_band),
            budget_usd: round2(ov.budget_usd),
            budget_pressure: ov.budget_usd > 0.0 && eom > ov.budget_usd * 0.9,
            children: vec![],
        }
    }

    /// Roll a parent's metrics up from its children once they're all attached.
    fn rollup_from_children(&mut self) {
        self.mtd_usd = round2(self.children.iter().map(|c| c.mtd_usd).sum());
        self.eom_forecast_usd = round2(self.children.iter().map(|c| c.eom_forecast_usd).sum());
        self.forecast_band = round2(self.children.iter().map(|c| c.forecast_band).sum());
        self.budget_usd = round2(self.children.iter().map(|c| c.budget_usd).sum());
        self.budget_pressure =
            self.budget_usd > 0.0 && self.eom_forecast_usd > self.budget_usd * 0.9;
    }
}

/// Assemble the company → group → manager → owner → project tree. A node's
/// metrics are the sum of its children; project leaves carry budget pressure
/// computed against their own forecast.
pub fn build_tree(facts: &[ProjectFact], overlay: &BTreeMap<String, ProjectOverlay>) -> CostNode {
    let mut company = CostNode {
        key: "company".into(),
        level: "company".into(),
        mtd_usd: 0.0,
        eom_forecast_usd: 0.0,
        forecast_band: 0.0,
        budget_usd: 0.0,
        budget_pressure: false,
        children: vec![],
    };
    // Nest by group → manager → owner, indexing children by key as we go.
    for f in facts {
        let default = ProjectOverlay::default();
        let ov = overlay.get(&f.project_id).unwrap_or(&default);
        let group = upsert_child(
            &mut company.children,
            &nz(&f.cost_group, "ungrouped"),
            "group",
        );
        let manager = upsert_child(
            &mut group.children,
            &nz(&f.manager, "unassigned"),
            "manager",
        );
        let owner = upsert_child(&mut manager.children, &nz(&f.owner, "unowned"), "owner");
        owner
            .children
            .push(CostNode::leaf(f.project_id.clone(), "project", f, ov));
    }
    // Roll metrics up from the leaves.
    for group in &mut company.children {
        for manager in &mut group.children {
            for owner in &mut manager.children {
                owner.rollup_from_children();
            }
            manager.rollup_from_children();
        }
        group.rollup_from_children();
    }
    company.rollup_from_children();
    company
}

/// Top movers by absolute percent change in MTD spend versus the previous
/// period, by project and by group. Sorted by `|Δ%|` descending.
#[derive(Debug, Clone, Serialize)]
pub struct Mover {
    pub key: String,
    pub current_usd: f64,
    pub previous_usd: f64,
    pub delta_usd: f64,
    pub delta_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Movers {
    pub by_project: Vec<Mover>,
    pub by_group: Vec<Mover>,
}

pub fn movers(current: &[ProjectFact], previous: &[ProjectFact], top: usize) -> Movers {
    let by_project = movers_for(
        fold(current, |f| f.project_id.clone()),
        fold(previous, |f| f.project_id.clone()),
        top,
    );
    let by_group = movers_for(
        fold(current, |f| nz(&f.cost_group, "ungrouped")),
        fold(previous, |f| nz(&f.cost_group, "ungrouped")),
        top,
    );
    Movers {
        by_project,
        by_group,
    }
}

/// Tagged-% is only honest with an account-total denominator (the Phase-2 seam).
/// Without one, `tagged_pct` is `None` ("n/a") rather than a misleading 100%.
#[derive(Debug, Clone, Serialize)]
pub struct TaggedReport {
    pub tagged_usd: f64,
    pub account_total_usd: Option<f64>,
    pub tagged_pct: Option<f64>,
}

pub fn tagged_report(tagged_usd: f64, account_total: Option<f64>) -> TaggedReport {
    let tagged_pct = account_total
        .filter(|t| *t > 0.0)
        .map(|t| round1(tagged_usd / t * 100.0));
    TaggedReport {
        tagged_usd: round2(tagged_usd),
        account_total_usd: account_total.map(round2),
        tagged_pct,
    }
}

fn movers_for(
    current: BTreeMap<String, f64>,
    previous: BTreeMap<String, f64>,
    top: usize,
) -> Vec<Mover> {
    let mut keys: Vec<&String> = current.keys().chain(previous.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out: Vec<Mover> = keys
        .into_iter()
        .map(|k| {
            let cur = *current.get(k).unwrap_or(&0.0);
            let prev = *previous.get(k).unwrap_or(&0.0);
            let delta = cur - prev;
            // New spend against a zero baseline is an infinite percent move; cap at
            // a large sentinel so it sorts to the top without being NaN/Inf.
            let pct = if prev > 0.0 {
                delta / prev * 100.0
            } else if cur > 0.0 {
                100.0
            } else {
                0.0
            };
            Mover {
                key: k.clone(),
                current_usd: round2(cur),
                previous_usd: round2(prev),
                delta_usd: round2(delta),
                delta_pct: round1(pct),
            }
        })
        .filter(|m| m.delta_usd.abs() > 0.0)
        .collect();
    out.sort_by(|a, b| {
        b.delta_pct
            .abs()
            .partial_cmp(&a.delta_pct.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(top);
    out
}

fn fold(facts: &[ProjectFact], key: impl Fn(&ProjectFact) -> String) -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    for f in facts {
        *m.entry(key(f)).or_insert(0.0) += f.mtd_usd;
    }
    m
}

fn upsert_child<'a>(children: &'a mut Vec<CostNode>, key: &str, level: &str) -> &'a mut CostNode {
    if let Some(idx) = children.iter().position(|c| c.key == key) {
        return &mut children[idx];
    }
    children.push(CostNode {
        key: key.to_string(),
        level: level.into(),
        mtd_usd: 0.0,
        eom_forecast_usd: 0.0,
        forecast_band: 0.0,
        budget_usd: 0.0,
        budget_pressure: false,
        children: vec![],
    });
    children.last_mut().unwrap()
}

fn nz(s: &str, fallback: &str) -> String {
    if s.trim().is_empty() {
        fallback.to_string()
    } else {
        s.to_string()
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(pid: &str, group: &str, manager: &str, owner: &str, mtd: f64) -> ProjectFact {
        ProjectFact {
            project_id: pid.into(),
            owner: owner.into(),
            manager: manager.into(),
            cost_group: group.into(),
            cost_center: "CC".into(),
            classification: "poc".into(),
            mtd_usd: mtd,
            estimated_usd: 0.0,
        }
    }

    #[test]
    fn tree_sums_children_up_the_org() {
        let facts = vec![
            fact("p1", "platform", "m1", "o1", 10.0),
            fact("p2", "platform", "m1", "o2", 5.0),
            fact("p3", "research", "m2", "o3", 20.0),
        ];
        let tree = build_tree(&facts, &BTreeMap::new());
        assert_eq!(tree.mtd_usd, 35.0);
        let platform = tree.children.iter().find(|c| c.key == "platform").unwrap();
        assert_eq!(platform.mtd_usd, 15.0);
        // platform → m1 → {o1,o2}
        let m1 = &platform.children[0];
        assert_eq!(m1.children.len(), 2, "two owners under one manager");
    }

    #[test]
    fn budget_pressure_trips_above_ninety_percent() {
        let facts = vec![fact("p1", "platform", "m1", "o1", 50.0)];
        let mut ov = BTreeMap::new();
        ov.insert(
            "p1".to_string(),
            ProjectOverlay {
                budget_usd: 100.0,
                eom_usd: 95.0,
                forecast_band: 5.0,
                has_forecast: true,
            },
        );
        let tree = build_tree(&facts, &ov);
        let leaf = &tree.children[0].children[0].children[0].children[0];
        assert!(leaf.budget_pressure, "95 forecast vs 100 budget is >90%");
    }

    #[test]
    fn movers_rank_by_absolute_percent_change() {
        let cur = vec![
            fact("p1", "platform", "m", "o", 30.0),
            fact("p2", "research", "m", "o", 10.0),
        ];
        let prev = vec![
            fact("p1", "platform", "m", "o", 10.0),
            fact("p2", "research", "m", "o", 10.0),
        ];
        let m = movers(&cur, &prev, 10);
        assert_eq!(m.by_project[0].key, "p1", "p1 tripled, p2 flat");
        assert_eq!(m.by_project.len(), 1, "flat p2 is filtered out");
    }

    #[test]
    fn tagged_pct_is_none_without_a_denominator() {
        assert!(tagged_report(100.0, None).tagged_pct.is_none());
        assert_eq!(tagged_report(50.0, Some(200.0)).tagged_pct, Some(25.0));
    }
}
