locals {
	has_minio = can(var.s3_providers["minio"])
}

resource "kubernetes_namespace" "minio" {
	metadata {
		name = "minio"
	}
}

module "minio_secrets" {
	source = "../modules/secrets"

	keys = ["s3/minio/root/key_id", "s3/minio/root/key"]
	optional = true
}

resource "helm_release" "minio" {
	count = local.has_minio ? 1 : 0

	name = "minio"
	namespace = kubernetes_namespace.minio.metadata.0.name
	repository = "oci://registry-1.docker.io/bitnamicharts"
	chart = "minio"
	version = "12.8.3"
	values = [yamlencode({
		replicaCount = 1
		auth = {
			rootUser = module.minio_secrets.values["s3/minio/root/key_id"]
			rootPassword = module.minio_secrets.values["s3/minio/root/key"]
			service = {
				nodePorts = {
					api = 92000
					console = 9201
				}
			}
		}
	})]
}

