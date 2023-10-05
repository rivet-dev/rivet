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
			cpu = 50
			cpu_cores = 0
			memory = 2000
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
	version = "51.2.0"
	values = [yamlencode({
		alertmanager = {
			alertManagerSpec = {
				storage = local.prometheus_storage
			}
		}
		prometheus = {
			prometheusSpec = {
				scrapeInterval = "15s"
				evaluationInterval = "15s"

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
		}
	})]
}

