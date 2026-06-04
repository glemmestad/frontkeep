output "client_id" {
  value = auth0_client.app.client_id
}

output "client_secret" {
  value     = data.auth0_client.app.client_secret
  sensitive = true
}
