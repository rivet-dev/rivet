resource "kubernetes_namespace" "vector" {
	count = var.prometheus_enabled ? 1 : 0

	metadata {
		name = "vector"
	}
}
