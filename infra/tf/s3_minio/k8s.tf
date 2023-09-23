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
		global = {
			storageClass = var.k8s_storage_class
		}
		replicaCount = 1
		auth = {
			rootUser = module.minio_secrets.values["s3/minio/root/key_id"]
			rootPassword = module.minio_secrets.values["s3/minio/root/key"]
		}
		service = {
			# Expose as LB so it can be accessed from the host if needed
			type = var.minio_port != null ? "LoadBalancer" : "ClusterIP"
		}
	})]
}

resource "kubectl_manifest" "minio_ingress_route" {
	depends_on = [helm_release.minio]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "IngressRoute"

		metadata = {
			name = "minio"
			namespace = kubernetes_namespace.minio.metadata.0.name
		}

		spec = {
			entryPoints = [ "websecure" ]

			routes = [
				{
					match = "Host(`minio.${var.domain_main}`)"
					kind  = "Rule"
					services = [
						{
							name = "minio"
							port = 9000
						}
					]
				}
			]

			tls = {
				secretName = "ingress-tls-cert"
				options = {
					name = "ingress-tls"
					namespace = "traefik"
				}
			}
		}
	})
}
