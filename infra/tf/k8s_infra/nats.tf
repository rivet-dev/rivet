resource "kubernetes_namespace_v1" "nats" {
	metadata {
		name = "nats"
	}
}

resource "helm_release" "nats" {
	depends_on = [kubernetes_namespace_v1.nats]

	name = "nats"
	namespace = "nats"
	repository = "https://nats-io.github.io/k8s/helm/charts/"
	chart = "nats"
	version = "1.0.0"
	values = [yamlencode({
		config = {
			cluster = {
				replicas = 1
			}
		}
	})]
}

