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
		statefulset = {
			replicas = 1
		}
		conf = {
			single-node = true
		}
		tls = {
			enabled = false

			# certs = {
			# 	provided = true
			# 	clientRootSecret = ""
			# 	nodeSecret = ""
			# }
		}
		storage = {
			persistentVolume = {
				storageClass = var.k8s_storage_class
			}
		}
	})]
}
