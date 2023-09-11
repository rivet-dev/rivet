# TODO: Create multiple of these

resource "kubernetes_namespace" "redis" {
	metadata {
		name = "redis"
	}
}

resource "helm_release" "redis" {
	name = "redis"
	namespace = kubernetes_namespace.redis.metadata.0.name
	repository = "https://charts.bitnami.com/bitnami"
	chart = "redis"
	version = "17.14.6"
	values = [yamlencode({
		global = {
			storageClass = var.k8s_storage_class
		}
		replica = {
			replicaCount = 1
		}
		auth = {
			enabled = false
		}
	})]
}

