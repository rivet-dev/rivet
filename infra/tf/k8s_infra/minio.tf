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
	repository = "https://charts.min.io/"
	chart = "minio"
	version = "5.0.13"
	values = [yamlencode({
		replicas = 1
		mode = "standalone"

		existingSecret = "minio-auth"
		
		minioAPIPort = 9200
		minioConoslePort = 9201
		service = {
			port = 9200
		}
		consoleService = {
			port = 9201
		}
	})]
}

resource "kubernetes_secret" "minio_auth" {
	depends_on = [kubernetes_namespace.minio]
	count = local.has_minio ? 1 : 0

	metadata {
		namespace = kubernetes_namespace.minio.metadata.0.name
		name = "minio-auth"
	}

	data = {
		rootUser = module.minio_secrets.values["s3/minio/root/key_id"]
		rootPassword = module.minio_secrets.values["s3/minio/root/key"]
	}
}
