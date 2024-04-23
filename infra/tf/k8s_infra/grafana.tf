locals {
	grafana_dashboards = {
		for f in fileset("${path.module}/grafana_dashboards/", "*.json"):
		"${trimsuffix(f, ".json")}" => {
			body = file("${path.module}/grafana_dashboards/${f}")
		}
	}
}

resource "kubernetes_config_map" "grafana_dashboard" {
	for_each = var.prometheus_enabled ? local.grafana_dashboards : {}

	metadata {
		namespace = kubernetes_namespace.prometheus.0.metadata.0.name
		name = "prometheus-rivet-${each.key}"
		labels = {
			grafana_dashboard = "1"
		}
	}

	data = {
		"${each.key}.json" = each.value.body
	}
}

