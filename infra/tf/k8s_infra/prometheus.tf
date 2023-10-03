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

