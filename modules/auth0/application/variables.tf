variable "name" {
  type = string
}

# Auth0 application type: non_interactive (M2M), spa, native, or regular_web.
variable "app_type" {
  type    = string
  default = "non_interactive"
}

variable "tags" {
  type    = map(string)
  default = {}
}
