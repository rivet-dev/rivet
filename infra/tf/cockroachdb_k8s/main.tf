resource "kubernetes_namespace" "cockroachdb" {
	metadata {
		name = "cockroachdb"
	}
}

resource "random_password" "root_password" {
	length = 32
	special = false
}

# NOTE: Helm chart is no longer supported by CockroachDB. However, it's intended to be used only for development and it's the easiest to set up.
resource "helm_release" "cockroachdb" {
	name = "cockroachdb"
	namespace = kubernetes_namespace.cockroachdb.metadata.0.name
	repository = "https://charts.cockroachdb.com/"
	chart = "cockroachdb"
	version = "11.1.5"  # v23.1.9
	values = [yamlencode({
		statefulset = {
			replicas = 1
		}
		conf = {
			single-node = true
		}
		storage = {
			persistentVolume = {
				storageClass = var.k8s_storage_class
			}
		}
		init = {
			provisioning = {
				users = [
					{
						user = "rivet-root"
						password = random_password.root_password.result
					}
				]
			}
		}
	})]
}

data "kubernetes_config_map" "root_ca" {
	depends_on = [helm_release.cockroachdb]

	metadata {
		name = "kube-root-ca.crt"
		namespace = kubernetes_namespace.cockroachdb.metadata.0.name
	}
}

