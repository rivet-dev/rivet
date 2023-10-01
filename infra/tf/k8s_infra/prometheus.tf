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
				storageSpec = local.prometheus_storage
			}
		}
	})]
}

