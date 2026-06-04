variable "name" {
  type = string
}

variable "region" {
  type    = string
  default = "us-west-2"
}

variable "tags" {
  type    = map(string)
  default = {}
}

# Existing network the service lands in. Asgard never creates a VPC; the operator
# supplies these from the account they are deploying into.
variable "vpc_id" {
  type = string
}

variable "subnet_ids" {
  type = list(string)
}

# Extra security groups attached to the service tasks. Empty ⇒ the module creates
# one that only accepts traffic from the ALB on container_port.
variable "security_group_ids" {
  type    = list(string)
  default = []
}

variable "image" {
  type = string
}

variable "cpu" {
  type    = string
  default = "512"
}

variable "memory" {
  type    = string
  default = "1024"
}

variable "desired_count" {
  type    = number
  default = 1
}

variable "container_port" {
  type    = number
  default = 8080
}

variable "health_path" {
  type    = string
  default = "/"
}

# Plain (non-secret) environment, injected as container `environment`.
variable "env" {
  type    = map(string)
  default = {}
}

# Secret environment: env var name -> Secrets Manager ARN (optionally suffixed
# `:json-key::` to pull one field). Injected as container `secrets`; the
# execution role is granted GetSecretValue on these automatically — the gap
# the predecessor platform hit, where a declared secret ref produced no working grant.
variable "secrets" {
  type    = map(string)
  default = {}
}

# Runtime task-role grants. Each list is built into the task-role inline policy,
# so a declared grant is always an effective grant (the predecessor silently dropped
# these). kms_decrypt closes the companion gap: a secret grant is inert without
# decrypt on the wrapping key.
variable "grants" {
  type = object({
    s3_read      = optional(list(string), [])
    s3_write     = optional(list(string), [])
    secrets_read = optional(list(string), [])
    kms_decrypt  = optional(list(string), [])
  })
  default = {}
}

# When set, an HTTPS listener is added on 443 with this ACM cert and HTTP 80
# redirects to it. Auth0 SPAs (auth0-spa-js) refuse any non-localhost HTTP
# origin, so HTTPS is required for a working login — the predecessor shipped HTTP-only.
variable "certificate_arn" {
  type    = string
  default = ""
}

# Optionally reuse a pre-approved execution role instead of creating one. Useful
# where an SCP forbids iam:PassRole on freshly created roles.
variable "execution_role_arn" {
  type    = string
  default = ""
}

# Whether the ALB is internet-facing. Internal by default.
variable "internal" {
  type    = bool
  default = true
}
