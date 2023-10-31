# resource "kubernetes_namespace" "loki" {
# 	metadata {
# 		name = "loki"
# 	}
# }

# resource "helm_release" "loki" {
# 	name = "loki"
# 	namespace = kubernetes_namespace.loki.metadata.0.name
# 	repository = "https://grafana.github.io/helm-charts"
# 	chart = "loki"
# 	version = "5.36.0"
# 	values = [yamlencode({
# 		loki = {
# 			auth_enabled = false
# 			commonConfig = {
# 				replication_factor = 1
# 			}
# 			storage = {
# 				type = "filesystem"
# 			}
# 		}
# 		singleBinary = {
# 			replicas = 1
# 			resources = var.limit_resources ? {
# 				limits = {
# 					cpu = "4"
# 					memory = "8192Mi"
# 				}
# 			} : null
# 		}
# 		monitoring = {
# 			lokiCanary = {
# 				enabled = true
# 				resources = var.limit_resources ? {
# 					limits = {
# 						cpu = "100m"
# 						memory = "200Mi"
# 					}
# 				} : null
# 			}
# 		}
# 		grafana-agent-operator = {
# 			resources = {
# 				limits = {
# 					cpu = "100m"
# 					memory = "200Mi"
# 				}
# 			}
# 		}
# 	})]
# }
