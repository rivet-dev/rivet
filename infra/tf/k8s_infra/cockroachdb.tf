resource "kubernetes_namespace" "cockroachdb" {
	metadata {
		name = "cockroachdb"
	}
}

resource "helm_release" "cockroachdb" {
	name = "cockroachdb"
	namespace = kubernetes_namespace.cockroachdb.metadata.0.name
	repository = "https://charts.cockroachdb.com/"
	chart = "cockroachdb"
	version = "11.1.5"
	values = [yamlencode({
		conf = {
			single-node = true
			statefulset = {
				# TODO: Doesn't work for some reason, still makes 3 replicas
				replicas = 1
			}
		}
		tls = {
			enabled = false
		}
	})]
}
