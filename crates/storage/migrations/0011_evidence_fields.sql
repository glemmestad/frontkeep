-- 0011: classification evidence record fields on projects_runtime.
-- The governance policy keys a ~30-field evidence record to classification tiers.
-- These columns store the fields beyond the core registration dims, all optional
-- at the column level (tier requirements are enforced in application logic, not
-- the schema). Multi-value fields (maintainers, critical_dependencies,
-- primary_data_flows) are JSON-text arrays. ALTER ADD COLUMN with a constant
-- default is portable across SQLite and Postgres.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)

ALTER TABLE projects_runtime ADD COLUMN requested_classification TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN repo_or_source_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN business_owner TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN technical_owner TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN team_or_org_of_record TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN support_contact TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN runbook_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN monitoring_or_logs_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN ci_status_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN critical_flow_test_or_eval_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN state_loss_posture TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN stack_exception TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN security_review_status TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN architecture_summary_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN incident_path TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN slo_or_service_target TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN rpo_rto TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN decommission_path TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN executive_accountable_owner TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN risk_acceptance_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN dr_exercise_evidence_url TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN audit_retention_requirement TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN recurring_review_date TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN maintainers TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN critical_dependencies TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN primary_data_flows TEXT NOT NULL DEFAULT '';
