resource "kubernetes_namespace" "nats" {
	metadata {
		name = "rivet-nats"
	}
}

resource "helm_release" "nats" {
	name = "nats"
	namespace = kubernetes_namespace.nats.metadata.0.name
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

