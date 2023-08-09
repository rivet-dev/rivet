# TODO: Create multiple of these

resource "kubernetes_namespace" "redis" {
	metadata {
		name = "redis"
	}
}

resource "helm_release" "redis" {
	depends_on = [kubernetes_namespace.redis]
	name = "redis"
	namespace = "redis"
	repository = "https://charts.bitnami.com/bitnami"
	chart = "redis"
	version = "17.14.6"
}

