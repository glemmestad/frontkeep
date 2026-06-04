# Optional Terraform module to run Asgard as a single container on a cloud VM or
# any Docker host. Kubernetes is supported but never required; this mirrors the
# headline `docker run` path for infra-as-code shops.

terraform {
  required_version = ">= 1.3"
  required_providers {
    docker = {
      source  = "kreuzwerker/docker"
      version = "~> 3.0"
    }
  }
}

resource "docker_image" "asgard" {
  name = "${var.image_repository}:${var.image_tag}"
}

resource "docker_volume" "data" {
  name = "${var.name}-data"
}

resource "docker_container" "asgard" {
  name  = var.name
  image = docker_image.asgard.image_id

  env = concat(
    [
      "ASGARD_DATABASE_URL=${var.database_url}",
      "ASGARD_BIND=0.0.0.0:8080",
    ],
    var.git_token == "" ? [] : ["ASGARD_GIT_TOKEN=${var.git_token}"],
  )

  ports {
    internal = 8080
    external = var.host_port
  }

  volumes {
    volume_name    = docker_volume.data.name
    container_path = "/data"
  }

  restart = "unless-stopped"
}
