locals {
	service_nats = lookup(var.services, "nats", {
		count = 1
		resources = {
			cpu = 1000
			memory = 1024
		}
	})
}

resource "kubernetes_namespace" "nats" {
	metadata {
		name = "nats"
	}
}

resource "helm_release" "nats" {
	name = "nats"
	namespace = kubernetes_namespace.nats.metadata.0.name
	repository = "https://nats-io.github.io/k8s/helm/charts/"
	chart = "nats"
	version = "1.0.0"
	values = [yamlencode({
		config = {
			cluster = {
				replicas = local.service_nats.count
			}
		}
		container = {
			env = {
				# See https://artifacthub.io/packages/helm/grafana/grafana#nats-container-resources
				GOMEMLIMIT = "${floor(local.service_nats.resources.memory * 0.9)}MiB"
			}
			merge = {
				resources = var.limit_resources ? {
					limits = {
						cpu = "${local.service_nats.resources.cpu}m"
						memory = "${local.service_nats.resources.memory}Mi"
					}
				} : null
			}
		}
		promExporter = {
			enabled = true
		}
	})]
}

