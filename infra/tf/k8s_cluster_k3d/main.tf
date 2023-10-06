terraform {
	required_providers {
		k3d = {
			source = "pvotal-tech/k3d"
			version = "0.0.7"
		}
	}
}

resource "k3d_cluster" "main" {
	name = "rivet-${var.namespace}"

	# Mount repository in to k3d so we can access the built binaries
	volume {
		source = var.project_root
		destination = "/rivet-src"
		node_filters = ["server:0"]
	}

	# Mount the /nix/store and /local since the build binaries depend on dynamic libs from there
	volume {
		source = "/nix/store"
		destination = "/nix/store"
		node_filters = ["server:0"]
	}

	volume {
		source = "/local"
		destination = "/local"
		node_filters = ["server:0"]
	}

	# HTTP
	port {
		host = "0.0.0.0"
		host_port = var.api_http_port
		container_port = 80
		node_filters = ["server:0"]
	}

	# HTTPS
	dynamic "port" {
		for_each = var.api_https_port != null ? [null] : []

		content {
			host = "0.0.0.0"
			host_port = var.api_https_port
			container_port = 443
			node_filters = ["server:0"]
		}

	}

	# Nomad (TCP)
	port {
		host = "0.0.0.0"
		host_port = var.nomad_port
		container_port = 5000
		node_filters = ["server:0"]
	}

	# api-route (TCP)
	port {
		host = "0.0.0.0"
		host_port = var.api_route_port
		container_port = 5001
		node_filters = ["server:0"]
	}

	# Minio
	dynamic "port" {
		for_each = var.minio_port != null ? [null] : []

		content {
			host = "0.0.0.0"
			host_port = var.minio_port
			container_port = 9000
			node_filters = ["server:0"]
		}

	}

	k3s {
		extra_args {
			arg = "--disable=traefik"
			node_filters = ["server:0"]
		}

		extra_args {
			arg = "--kubelet-arg=max-pods=256"
			node_filters = ["server:0"]
		}
	}
}

