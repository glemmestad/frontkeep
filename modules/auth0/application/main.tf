# An Auth0 application (client) in an existing tenant. The auth0/auth0 provider
# reads AUTH0_DOMAIN / AUTH0_CLIENT_ID / AUTH0_CLIENT_SECRET from the inherited
# process env (Asgard loads them from .env). The project `tags` map is stamped
# onto the client's metadata so `project=<id>` attribution lands downstream.

terraform {
  required_providers {
    auth0 = { source = "auth0/auth0" }
  }
}

provider "auth0" {}

resource "auth0_client" "app" {
  name     = var.name
  app_type = var.app_type

  # Auth0 client_metadata values must be strings; the tags map already is.
  client_metadata = var.tags
}

# The auth0_client resource no longer exports the secret (provider v1+); the data
# source reads it back (needs Management API read:client_keys at runtime).
data "auth0_client" "app" {
  client_id = auth0_client.app.client_id
}
