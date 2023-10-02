resource "kubernetes_namespace" "loki" {
	metadata {
		name = "loki"
	}
}

resource "helm_release" "loki" {
	name = "loki"
	namespace = kubernetes_namespace.loki.metadata.0.name
	repository = "https://grafana.github.io/helm-charts"
	chart = "loki"
	version = "5.18.0"
	values = [yamlencode({
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
