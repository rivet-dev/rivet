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
	version = "5.18.0"
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
		singleBinary = {
			replicas = 1
		}
	})]
}
