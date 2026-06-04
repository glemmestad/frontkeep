output "container_name" {
  value = docker_container.asgard.name
}

output "endpoint" {
  value = "http://localhost:${var.host_port}"
}
