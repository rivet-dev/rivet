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

	crdb_host = "${try(data.terraform_remote_state.cockroachdb_k8s.outputs.host, data.terraform_remote_state.cockroachdb_managed.outputs.host)}:${try(data.terraform_remote_state.cockroachdb_k8s.outputs.port, data.terraform_remote_state.cockroachdb_managed.outputs.port)}"
}

module "crdb_user_grafana_secrets" {
	source = "../modules/secrets"

	keys = [ "crdb/user/grafana/username", "crdb/user/grafana/password" ]
}

resource "helm_release" "grafana" {
	name = "grafana"
	namespace = "grafana"
	repository = "https://grafana.github.io/helm-charts"
	chart = "grafana"
	version = "7.3.9"
	values = [yamlencode({
		priorityClassName = "monitoring-priority"
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
				name = "crdb-ca"
				configMap = "crdb-ca"
				mountPath = "/local/crdb/ca.crt"
				subPath = "ca.crt"
				readOnly = true
			}
		]

		sidecar = {
			dashboards = {
				enabled = true
				searchNamespace = ["grafana", "prometheus"]
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
	for_each = local.grafana_dashboards

	metadata {
		namespace = "grafana"
		name = "grafana-rivet-${each.key}"
		labels = {
			grafana_dashboard = "1"
		}
	}

	data = {
		"${each.key}.json" = each.value.body
	}
}
