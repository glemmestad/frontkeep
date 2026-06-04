# Databricks Workflow Job

Provisions a `databricks_job` (single notebook task on serverless compute) via the
terraform connector + the Databricks provider. Review-tier (recurring compute).

**Spec fields**: `name`, `notebook_path` (a workspace notebook path), optional
`schedule_cron` (Quartz cron; omit for trigger-only) and `timezone`. The `project`
tag is stamped on the job, propagating to billing for per-project attribution.

**Outputs**: `job_id`, `job_url`.
