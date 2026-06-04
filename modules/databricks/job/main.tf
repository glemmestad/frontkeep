# A Databricks Workflows job running a single notebook task on serverless compute,
# optionally on a schedule. Provider creds come from the inherited env. The project
# `tags` map is attached to the job (propagates to billing for attribution).

terraform {
  required_providers {
    databricks = { source = "databricks/databricks" }
  }
}

provider "databricks" {}

variable "name" {
  type = string
}

# Workspace path of the notebook to run, e.g. /Workspace/Repos/team/etl.
variable "notebook_path" {
  type = string
}

# Quartz cron expression; empty = trigger/manual only (no schedule).
variable "schedule_cron" {
  type    = string
  default = ""
}

variable "timezone" {
  type    = string
  default = "UTC"
}

variable "tags" {
  type    = map(string)
  default = {}
}

resource "databricks_job" "this" {
  name = var.name

  task {
    task_key = "main"
    notebook_task {
      notebook_path = var.notebook_path
    }
  }

  dynamic "schedule" {
    for_each = var.schedule_cron == "" ? [] : [1]
    content {
      quartz_cron_expression = var.schedule_cron
      timezone_id            = var.timezone
    }
  }

  tags = var.tags
}

output "job_id" {
  value = databricks_job.this.id
}

output "job_url" {
  value = databricks_job.this.url
}
