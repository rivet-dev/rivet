terraform {
	required_providers {
		kubectl = {
			source = "gavinbunney/kubectl"
			version = "1.14.0"
		}
	}
}

locals {
	entrypoints = var.tls_enabled ? {
		"web" = { tls = null }
		"websecure" = {
			tls = {
				secretName = "ingress-tls-cert"
				options = {
					name = "ingress-tls"
					namespace = kubernetes_namespace.traefik.metadata[0].name
				}
			}
		}
	} : {
		"web" = { tls = null }
	}
}

