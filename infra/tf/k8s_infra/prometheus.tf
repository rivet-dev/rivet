locals {
	prometheus_storage = {
		volumeClaimTemplate = {
			spec = {
				storageClassName = var.k8s_storage_class
				resources = {
					requests = {
						# TODO: Allow configuring
						storage = "10Gi"
					}
				}
			}
		}
	}
	service_prometheus = lookup(var.services, "prometheus", {
		count = 1
		resources = {
			cpu = 500
			cpu_cores = 0
			memory = 2500
		}
	})
}

resource "kubernetes_namespace" "prometheus" {
	metadata {
		name = "prometheus"
	}
}

resource "helm_release" "prometheus" {
	name = "prometheus"
	namespace = kubernetes_namespace.prometheus.metadata.0.name
	repository = "https://prometheus-community.github.io/helm-charts"
	chart = "kube-prometheus-stack"
	version = "51.5.1"
	values = [yamlencode({
		alertmanager = {
			alertmanagerSpec = {
				storage = local.prometheus_storage

				# Need to downgrade the alertmanager version from 0.26.0
				# https://github.com/grafana/grafana/issues/71364
				# https://github.com/prometheus/alertmanager/issues/3505
				image = {
					registry = "quay.io"
					repository = "prometheus/alertmanager"
					tag = "v0.25.1"
				}


				# Use resource for config instead of helm chart
				# alertmanagerConfiguration = {
				# 	name = "alertmanager-config"
				# }
			}
				
			# Some values copied from default helm chart
			config = {
				global = {
					resolve_timeout = "5m"
				}
				inhibit_rules = [
					{
						source_matchers = [
							"severity = critical",
						]
						target_matchers = [
							"severity =~ warning|info",
						]
						equal = ["namespace", "alertname"]
					},
					{
						source_matchers = [
							"severity = warning",
						]
						target_matchers = [
							"severity = info",
						]
						equal = ["namespace", "alertname"]
					},
					{
						source_matchers = [
							"alertname = \"InfoInhibitor\"",
						]
						target_matchers = [
							"severity = info",
						]
						equal = ["namespace"]
					}
				]
				route = {
					group_by = ["namespace"]
					group_wait = "15s"
					group_interval = "1m"
					repeat_interval = "4h"
					receiver = "slack"
					routes = [
						{
							receiver = "null"
							matchers = [
								"alertname =~ \"InfoInhibitor|Watchdog\""
							]
						}
					]
				}
				receivers = [
					{
						name = "null"
					},
					{
						name = "slack"
						slack_configs = [
							{
								channel = "C060NE3S536"
								api_url = "https://hooks.slack.com/services/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"
							}
						]
					}
				]
				templates = [
					"/etc/alertmanager/config/*.tmpl"
				]
			}
		}

		prometheus = {
			prometheusSpec = {
				scrapeInterval = "15s"
				evaluationInterval = "15s"

				# additionalArgs = [{
				# 	name = "log.level"
				# 	value = "debug"
				# }]

				storageSpec = local.prometheus_storage
			
				resources = {
					limits = {
						memory = "${local.service_prometheus.resources.memory}Mi"
						cpu = (
							local.service_prometheus.resources.cpu_cores > 0 ?
							"${local.service_prometheus.resources.cpu_cores * 1000}m"
							: "${local.service_prometheus.resources.cpu}m"
						)
					}
				}

				# Monitor all namespaces
				podMonitorNamespaceSelector = { any = true }
				podMonitorSelector = {}
				podMonitorSelectorNilUsesHelmValues = false
				ruleNamespaceSelector = { any = true }
				ruleSelector = {}
				ruleSelectorNilUsesHelmValues = false
				serviceMonitorNamespaceSelector = { any = true }
				serviceMonitorSelector = {}
				serviceMonitorSelectorNilUsesHelmValues = false
			}
		}

		grafana = {
			additionalDataSources = [
				{
					name = "Loki"
					type = "loki"
					uid = "loki"
					url = "http://loki-gateway.loki.svc.cluster.local:80/"
					access = "proxy"
					jsonData = {}
				}
			]

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
		}

		defaultRules = {
			disabled = {
				KubeProxyDown = true
				KubeControllerManagerDown = true
				KubeSchedulerDown = true
				CPUThrottlingHigh = true
			}
		}
	})]
}

# resource "kubectl_manifest" "test_rule" {
# 	depends_on = [helm_release.prometheus]

# 	yaml_body = yamlencode({
# 		apiVersion = "monitoring.coreos.com/v1"
# 		kind = "PrometheusRule"
# 		metadata = {
# 			name = "prometheus-example-rules"
# 			namespace = kubernetes_namespace.prometheus.metadata.0.name
# 		}
# 		spec = {
# 			groups = [
# 				{
# 					name = "test-group2"
# 					interval = "1s"
# 					rules = [
# 						{
# 							alert = "ExampleAlert2"
# 							annotations = {
# 								message = "test message {{ $value }}"
# 								# message = <<-EOF
# 								# {{ $labels.job }} {{ $labels.route }} is experiencing {{ printf
# 								# "%.2f" $value }}% errors.
# 								# EOF
# 							}
# 							# expr = <<-EOF
# 							# 	sum by (service)
# 							# 	(increase(rivet_chirp_request_duration_count{error_code!="",error_code!~"(1002|VALIDATION_ERROR)"}
# 							# 	[2m]))

# 							# 	/

# 							# 	sum by (service)
# 							# 	(clamp_min(increase(rivet_chirp_request_duration_count{error_code!~"(1002|VALIDATION_ERROR)"}
# 							# 	[2m]), 1))
# 							# 	EOF
# 							expr = "vector(1)"
# 							# for = "15m"
# 							labels = {
# 								severity = "critical"
# 							}
# 						}
# 					]
# 				}
# 			]
# 		}
# 	})
# }
