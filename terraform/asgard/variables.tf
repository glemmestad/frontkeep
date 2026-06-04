variable "name" {
  type    = string
  default = "asgard"
}

variable "image_repository" {
  type    = string
  default = "ghcr.io/asgard/asgard"
}

variable "image_tag" {
  type    = string
  default = "latest"
}

variable "database_url" {
  type        = string
  description = "sqlite:///data/asgard.db by default; set a postgres:// DSN to scale out."
  default     = "sqlite:///data/asgard.db"
}

variable "git_token" {
  type      = string
  default   = ""
  sensitive = true
}

variable "host_port" {
  type    = number
  default = 8080
}
