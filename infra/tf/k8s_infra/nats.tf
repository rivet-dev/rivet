locals {
	service_nats = lookup(var.services, "nats", {
		count = var.deploy_method_cluster ? 3 : 1
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

resource "kubernetes_priority_class" "nats_priority" {
	metadata {
		name = "nats-priority"
	}

	value = 40
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
		podTemplate = {
			merge = {
				priorityClassName = kubernetes_priority_class.nats_priority.metadata.0.name
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

