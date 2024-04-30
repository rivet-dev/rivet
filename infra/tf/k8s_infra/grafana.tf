locals {
	service_grafana = lookup(var.services, "grafana", {
		count = 1
		resources = {
			cpu = 500
			memory = 512
		}
	})

	grafana_dashboards = {
		for f in fileset("${path.module}/grafana_dashboards/", "*.json"):
		"${trimsuffix(f, ".json")}" => {
			body = file("${path.module}/grafana_dashboards/${f}")
		}
	}
}

resource "kubernetes_namespace" "grafana" {
	count = var.prometheus_enabled ? 1 : 0

	metadata {
		name = "grafana"
	}
}

resource "helm_release" "grafana" {
	count = var.prometheus_enabled ? 1 : 0
	depends_on = [helm_release.vpa]

	name = "grafana"
	namespace = kubernetes_namespace.grafana.0.metadata.0.name
	repository = "https://grafana.github.io/helm-charts"
	chart = "grafana"
	version = "7.3.9"
	values = [yamlencode({
		"grafana.ini" = {
			auth = {
				disable_login_form = true
			}
			"auth.anonymous" = {
				enabled = true
				org_role = "Admin"
			}
		}	

		resources = var.limit_resources ? {
			limits = {
				memory = "${local.service_grafana.resources.memory}Mi"
				cpu = "${local.service_grafana.resources.cpu}m"
			}
		} : null

		datasources = {
			"datasources.yaml" = {
				apiVersion = 1

				datasources = [
					{
						name = "Prometheus"
						type = "prometheus"
						uid = "prometheus"
						url = "http://prometheus-kube-prometheus-prometheus.prometheus:9090/"
						access = "proxy"
						isDefault = true
						jsonData = {
							httpMethod = "POST"
							# prometheus.prometheusSpec.scrapeInterval
							timeInterval = "30s"
						}
					},
					{
						name = "Loki"
						type = "loki"
						uid = "loki"
						url = "http://loki-gateway.loki.svc.cluster.local:80/"
						access = "proxy"
						jsonData = {}
					},
					{
						name = "CockroachDB"
						type = "postgres"
						uid = "crdb"
						url = local.crdb_host
						user = module.crdb_user_grafana_secrets.values["crdb/user/grafana/username"]
						secureJsonData = {
							password = module.crdb_user_grafana_secrets.values["crdb/user/grafana/password"]
						}
						jsonData = {
							sslmode = "verify-ca"
							sslRootCertFile = "/local/crdb/ca.crt"
						}
						secret = true
					}
				]
			}
		}

		extraConfigmapMounts = [
			# TLS Cert for postgres datasource
			{
				name = kubernetes_config_map.crdb_ca["grafana"].metadata.0.name
				configMap = "crdb-ca"
				mountPath = "/local/crdb/ca.crt"
				subPath = "ca.crt"
				readOnly = true
			}
		]

		sidecar = {
			dashboards = {
				enabled = true
			}
		}

		serviceMonitor = {
			enabled = true
			path = "/metrics"
			labels = {}

			interval = ""
			scheme = "http"
			tlsConfig = {}
			scrapeTimeout = "15s"

			relabelings = []
		}
	})]
}

resource "kubernetes_config_map" "grafana_dashboard" {
	for_each = var.prometheus_enabled ? local.grafana_dashboards : {}

	metadata {
		namespace = kubernetes_namespace.grafana.0.metadata.0.name
		name = "grafana-rivet-${each.key}"
		labels = {
			grafana_dashboard = "1"
		}
	}

	data = {
		"${each.key}.json" = each.value.body
	}
}
