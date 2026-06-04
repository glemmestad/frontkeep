# Cloud-free Terraform module used to exercise the universal `terraform`
# connector in CI: no provider, no remote API. It records the injected name +
# project tags and emits one sensitive output to prove secret values are routed
# to the secret store (and never land in the resource record).

variable "name" {
  type    = string
  default = "demo"
}

variable "tags" {
  type    = map(string)
  default = {}
}

resource "terraform_data" "marker" {
  input = {
    name    = var.name
    project = lookup(var.tags, "project", "")
  }
}

output "resource_name" {
  value = var.name
}

output "project" {
  value = lookup(var.tags, "project", "")
}

output "token" {
  value     = "tok-${var.name}-${lookup(var.tags, "project", "none")}"
  sensitive = true
}
