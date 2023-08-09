resource "kubernetes_namespace" "nats" {
	metadata {
		name = "nats"
	}
}

resource "helm_release" "nats" {
	depends_on = [kubernetes_namespace.nats]

	name = "nats"
	namespace = "nats"
	repository = "https://nats-io.github.io/k8s/helm/charts/"
	chart = "nats"
	version = "1.0.0"
}

