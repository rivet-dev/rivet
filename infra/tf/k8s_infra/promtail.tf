locals {
	labels = {
		ns = var.namespace
		service = "rivet-$(KUBERNETES_POD_NAME)"
		node = "$(KUBERNETES_NODE_NAME)"
		alloc = "$(KUBERNETES_POD_ID)"
	}
	external_labels = join(",", [
		for key, value in local.labels:
		"${key}=${value}"
	])
}

resource "kubernetes_namespace" "promtail" {
	metadata {
		name = "promtail"
	}
}

resource "helm_release" "promtail" {
	name = "promtail"
	namespace = kubernetes_namespace.promtail.metadata.0.name
	repository = "https://grafana.github.io/helm-charts"
	chart = "promtail"
	version = "6.15.1"
	values = [yamlencode({
		config = {
			clients = [{
				url = "http://loki.loki.svc.cluster.local:3100/loki/api/v1/push"
				tenant_id = 1

				# basic_auth = {
				# 	username = loki
				# 	password = secret
				# }
			}]
			extraRelabelConfigs = [{
				action = "labeldrop"
				regex = "^(host|filename)$"
			}]
		}

		extraArgs = [
			"-client.external-labels=${local.external_labels}"
		]
	})]
}
