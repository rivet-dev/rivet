locals {
	service_prometheus = lookup(var.services, "prometheus", {
		count = var.deploy_method_cluster ? 2 : 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})

	# TODO: Allow configuring
	# Use exact bytes so we don't have to worry about converting between mebibytes in k8s and megabytes in Prometheus
	prometheus_storage = 64 * 1000 * 1000 * 1000 # Bytes

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

	has_slack_receiver = (
		module.alertmanager_secrets.values["alertmanager/slack/url"] != "" &&
		module.alertmanager_secrets.values["alertmanager/slack/channel"] != ""
	)

	receivers = flatten([
		[{
			name = "null"
		}],
		local.has_slack_receiver ? [{
			name = "slack"
			slack_configs = [
				{
					channel = module.alertmanager_secrets.values["alertmanager/slack/channel"]
					api_url = module.alertmanager_secrets.values["alertmanager/slack/url"]
					send_resolved = true
				}
			]
		}] : []
	])
}

module "alertmanager_secrets" {
	source = "../modules/secrets"

	keys = ["alertmanager/slack/url", "alertmanager/slack/channel"]
	optional = true
}

module "crdb_user_grafana_secrets" {
	source = "../modules/secrets"

	keys = [ "crdb/user/grafana/username", "crdb/user/grafana/password" ]
}

resource "kubernetes_namespace" "prometheus" {
	count = var.prometheus_enabled ? 1 : 0

	metadata {
		name = "prometheus"
	}
}

resource "helm_release" "prometheus" {
	count = var.prometheus_enabled ? 1 : 0
	depends_on = [helm_release.vpa]

	name = "prometheus"
	namespace = kubernetes_namespace.prometheus.0.metadata.0.name
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
			priorityClassName = kubernetes_priority_class.daemon_priority.metadata.0.name
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
				replicas = local.service_alertmanager.count

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
					}
				]
				route = {
					group_by = ["namespace"]
					group_wait = "15s"
					group_interval = "1m"
					repeat_interval = "4h"
					receiver = "null"
					routes = flatten([
						[{
							receiver = "null"
							matchers = [
								"alertname = Watchdog"
							]
						}],
						local.has_slack_receiver ? [{
							receiver = "slack"
							matchers = [
								"severity =~ warning|critical"
							]
						}] : []
					])
				}
				receivers = local.receivers
				templates = [
					"/etc/alertmanager/config/*.tmpl"
				]
			}
		}

		prometheus = {
			prometheusSpec = {
				replicas = local.service_prometheus.count
				scrapeInterval = "15s"
				evaluationInterval = "15s"

				retention = "7d"

				# Provide unused storage space
				#
				# See https://arc.net/l/quote/ncocbkuo
				retentionSize = "${floor(local.prometheus_storage * 0.8)}B"

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
									storage = "${local.prometheus_storage}"
								}
							}
						}
					}
				}
			
				priorityClassName = kubernetes_priority_class.monitoring_priority.metadata.0.name
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
				# TODO: Re-enable these alerts
				KubeProxyDown = true
				KubeControllerManagerDown = true
				KubeSchedulerDown = true
				CPUThrottlingHigh = true
				KubeJobNotCompleted = true
				PrometheusOutOfOrderTimestamps = true

				# See https://github.com/prometheus-community/helm-charts/issues/1773#issue-1126092733
				InfoInhibitor = true
			}
		}

		# Configured in grafana.tf
		grafana = {
			enabled = false
			forceDeployDashboards = true
		}
	})]
}
