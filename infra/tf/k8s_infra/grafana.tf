resource "kubernetes_namespace" "grafana" {
	count = var.prometheus_enabled ? 1 : 0

	metadata {
		name = "grafana"
	}
}

