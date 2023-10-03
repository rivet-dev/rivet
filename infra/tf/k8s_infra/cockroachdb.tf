locals {
	cockroachdb_k8s = var.cockroachdb_provider == "kubernetes"
}

module "crdb_secrets" {
	count = local.cockroachdb_k8s ? 1 : 0

	source = "../modules/secrets"

	keys = [ "crdb/username", "crdb/password" ]
}

resource "kubernetes_namespace" "cockroachdb" {
	count = local.cockroachdb_k8s ? 1 : 0

	metadata {
		name = "cockroachdb"
	}
}

# NOTE: Helm chart is no longer supported by CockroachDB. However, it's intended to be used only for development and it's the easiest to set up.
resource "helm_release" "cockroachdb" {
	count = local.cockroachdb_k8s ? 1 : 0

	name = "cockroachdb"
	namespace = kubernetes_namespace.cockroachdb[0].metadata.0.name
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
		tls = {
			enabled = true
		}
		storage = {
			persistentVolume = {
				storageClass = var.k8s_storage_class
			}
		}
		init = {
			provisioning = {
				enabled = true
				users = [
					{
						name = module.crdb_secrets[0].values["crdb/username"]
						password = module.crdb_secrets[0].values["crdb/password"]
						options = ["CREATEDB"]
					}
				]
			}
		}

		serviceMonitor = {
			enabled = true
			namespace = kubernetes_namespace.cockroachdb[0].metadata.0.name
		}
	})]
}

data "kubernetes_secret" "crdb_ca" {
	count = local.cockroachdb_k8s ? 1 : 0

	depends_on = [helm_release.cockroachdb]

	metadata {
		name = "cockroachdb-ca-secret"
		namespace = kubernetes_namespace.cockroachdb[0].metadata.0.name
	}
}

resource "kubernetes_config_map" "crdb_ca" {
	for_each = local.cockroachdb_k8s ? toset(["rivet-service", "bolt"]) : toset([])

	metadata {
		name = "crdb-ca"
		namespace = each.value
	}

	data = {
		"ca.crt" = data.kubernetes_secret.crdb_ca[0].data["ca.crt"]
	}
}
