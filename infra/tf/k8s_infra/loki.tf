locals {
	service_loki = lookup(var.services, "loki", {
		count = 1
		resources = {
			cpu = 2000
			memory = 2048
		}
	})
}

resource "kubernetes_namespace" "loki" {
	metadata {
		name = "loki"
	}
}

resource "kubernetes_priority_class" "loki_priority" {
	metadata {
		name = "loki-priority"
	}

	value = 40
}

resource "helm_release" "loki" {
	name = "loki"
	namespace = kubernetes_namespace.loki.metadata.0.name
	repository = "https://grafana.github.io/helm-charts"
	chart = "loki"
	version = "5.36.0"
	values = [yamlencode({
		global = {
			priorityClassName = kubernetes_priority_class.loki_priority.metadata.0.name
		}
		loki = {
			auth_enabled = false
			commonConfig = {
				replication_factor = 1
			}
			storage = {
				type = "filesystem"
			}
		}
		compactor = {
			retention_enabled = true
			retention_delete_delay = "2h"
			retention_delete_worker_count = 150
		}
		tableManager = {
			retention_deletes_enabled = true
			# NOTE: This must be a multiple of the index period in `schemaConfig`. Default is 24h
			# (https://github.com/grafana/loki/blob/main/cmd/loki/loki-local-config.yaml#L34)
			retention_period = "24h"
		}
		singleBinary = {
			replicas = 1
			resources = var.limit_resources ? {
				limits = {
					memory = "${local.service_loki.resources.memory}Mi"
					cpu = "${local.service_loki.resources.cpu}m"
				}
			} : null
			persistence = {
				size = var.deploy_method_cluster ? "128Gi" : "10Gi"
				storageClass = var.k8s_storage_class
			}
		}
		monitoring = {
			lokiCanary = {
				enabled = true
				resources = var.limit_resources ? {
					limits = {
						cpu = "100m"
						memory = "200Mi"
					}
				} : null
			}
		}
		grafana-agent-operator = {
			resources = {
				limits = {
					cpu = "100m"
					memory = "200Mi"
				}
			}
		}
		monitoring = {
			dashboards = {
				namespace = kubernetes_namespace.prometheus.metadata.0.name
			}
			rules = {
				namespace = kubernetes_namespace.prometheus.metadata.0.name
			}
		}
	})]
}
