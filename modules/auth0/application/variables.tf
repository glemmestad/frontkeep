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

# Existing tenant connections to enable on this client (e.g. an enterprise SSO
# connection). Empty = the client keeps whatever the tenant enables by default,
# so an OSS deploy that never sets this is unchanged. Referenced by connection
# name; the connection itself must already exist in the tenant.
variable "enabled_connections" {
  type    = list(string)
  default = []
}

# OAuth redirect/CORS/logout URLs for the client. Optional list attributes on
# auth0_client; empty (the default) keeps an OSS deploy unchanged. For a `spa`
# these must be populated for a browser login round-trip to complete, which is
# why the collab recipe passes them once the app's URL exists.
variable "callbacks" {
  type    = list(string)
  default = []
}

variable "web_origins" {
  type    = list(string)
  default = []
}

variable "allowed_logout_urls" {
  type    = list(string)
  default = []
}

# When set, create a dedicated resource server (API) and use its identifier as
# the application's audience. `{project}` is substituted with the project id so
# each project gets a stable, unique audience. Empty = no API is created and the
# `audience` output is empty.
variable "resource_server_template" {
  type    = string
  default = ""
}
