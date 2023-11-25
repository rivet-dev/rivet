locals {
	service_prometheus = lookup(var.services, "prometheus", {
		count = 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})

	# TODO: Allow configuring
	prometheus_storage = 64000 # Mebibytes

	service_alertmanager = lookup(var.services, "alertmanager", {
		count = 1
		resources = {
			cpu = 250
			memory = 250
		}
	})

	service_prometheus_operator = lookup(var.services, "prometheus-operator", {
		count = 1
		resources = {
			cpu = 200
			memory = 200
		}
	})

	service_node_exporter = lookup(var.services, "node-exporter", {
		count = 1
		resources = {
			cpu = 200
			memory = 50
		}
	})

	service_kube_state_metrics = lookup(var.services, "kube-state-metrics", {
		count = 1
		resources = {
			cpu = 100
			memory = 64
		}
	})

	service_grafana = lookup(var.services, "grafana", {
		count = 1
		resources = {
			cpu = 500
			memory = 512
		}
	})

	has_slack_receiver = (
		module.alertmanager_secrets.values["alertmanager/slack/url"] != "" &&
		module.alertmanager_secrets.values["alertmanager/slack/channel"] != ""
	)

	_receivers = [
		{
			name = "null"
		},
		local.has_slack_receiver ? {
			name = "slack"
			slack_configs = [
				{
					channel = module.alertmanager_secrets.values["alertmanager/slack/channel"]
					api_url = module.alertmanager_secrets.values["alertmanager/slack/url"]
					send_resolved = true
				}
			]
		} : null
	]
	receivers = [ for v in local._receivers : v if v != null ]
}

module "alertmanager_secrets" {
	source = "../modules/secrets"

	keys = ["alertmanager/slack/url", "alertmanager/slack/channel"]
	optional = true
}

resource "kubernetes_namespace" "prometheus" {
	metadata {
		name = "prometheus"
	}
}

# Set a high priority for Node Exporter so it can run on all nodes
resource "kubernetes_priority_class" "node_exporter_priority" {
	metadata {
		name = "node-exporter-priority"
	}
	value = 90
}

resource "kubernetes_priority_class" "prometheus_priority" {
	metadata {
		name = "prometheus-priority"
	}
	value = 40
}

resource "helm_release" "prometheus" {
	name = "prometheus"
	namespace = kubernetes_namespace.prometheus.metadata.0.name
	repository = "https://prometheus-community.github.io/helm-charts"
	chart = "kube-prometheus-stack"
	version = "51.5.1"
	values = [yamlencode({
		prometheus-node-exporter = {
			resources = var.limit_resources ? {
				limits = {
					memory = "${local.service_node_exporter.resources.memory}Mi"
					cpu = "${local.service_node_exporter.resources.cpu}m"
				}
			} : null
			priorityClassName = kubernetes_priority_class.node_exporter_priority.metadata.0.name
			affinity = {
				nodeAffinity = {
					requiredDuringSchedulingIgnoredDuringExecution = {
						nodeSelectorTerms = [{
							matchExpressions = [{
								key = "eks.amazonaws.com/compute-type"
								operator = "NotIn"
								values = ["fargate"]
							}]
						}]
					}
				}
			}
		}
		kube-state-metrics = {
			resources = var.limit_resources ? {
				limits = {
					memory = "${local.service_kube_state_metrics.resources.memory}Mi"
					cpu = "${local.service_kube_state_metrics.resources.cpu}m"
				}
			} : null
		}
		alertmanager = {
			alertmanagerSpec = {
				storage = {
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

				resources = var.limit_resources ? {
					limits = {
						memory = "${local.service_alertmanager.resources.memory}Mi"
						cpu = "${local.service_alertmanager.resources.cpu}m"
					}
				} : null

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
					# Disable info alerts
					#
					# See https://github.com/prometheus-community/helm-charts/issues/1773 for
					# why we don't use InfoInhibitor
					{
						source_matchers = [
							"severity = info",
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
					receiver = local.has_slack_receiver ? "slack" : "null"
					routes = [
						{
							receiver = "null"
							matchers = [
								"alertname = Watchdog"
							]
						}
					]
				}
				receivers = local.receivers
				templates = [
					"/etc/alertmanager/config/*.tmpl"
				]
			}
		}

		prometheus = {
			prometheusSpec = {
				scrapeInterval = "15s"
				evaluationInterval = "15s"

				retention = "7d"
				retentionSize = "${local.prometheus_storage - 100}MB"

				# additionalArgs = [{
				# 	name = "log.level"
				# 	value = "debug"
				# }]

				storageSpec = {
					volumeClaimTemplate = {
						spec = {
							storageClassName = var.k8s_storage_class
							resources = {
								requests = {
									storage = "${local.prometheus_storage}Mi"
								}
							}
						}
					}
				}
			
				priorityClassName = kubernetes_priority_class.prometheus_priority.metadata.0.name
				resources = var.limit_resources ? {
					limits = {
						memory = "${local.service_prometheus.resources.memory}Mi"
						cpu = "${local.service_prometheus.resources.cpu}m"
					}
				} : null

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

		prometheusOperator = {
			resources = var.limit_resources ? {
				limits = {
					memory = "${local.service_prometheus_operator.resources.memory}Mi"
					cpu = "${local.service_prometheus_operator.resources.cpu}m"
				}
			} : null
		}

		defaultRules = {
			disabled = {
				KubeProxyDown = true
				KubeControllerManagerDown = true
				KubeSchedulerDown = true
				CPUThrottlingHigh = true
				KubeJobNotCompleted = true
				InfoInhibitor = true
			}
		}

		grafana = {
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
	})]
}
