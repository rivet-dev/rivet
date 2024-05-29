resource "kubernetes_namespace" "vector" {
	count = var.prometheus_enabled ? 1 : 0

	metadata {
		name = "vector"
	}
}

module "vector_secrets" {
	source = "../modules/secrets"

	keys = [
		"vector/http/username",
		"vector/http/password",
	]
}

resource "kubectl_manifest" "vector_ingress_route" {
	for_each = var.prometheus_enabled ? local.entrypoints : {}

	depends_on = [null_resource.daemons, kubectl_manifest.vector_basic_auth]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "IngressRoute"

		metadata = {
			name = "vector-${each.key}"
			namespace = kubernetes_namespace.vector.0.metadata.0.name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			entryPoints = [ each.key ]

			routes = [
				{
					kind  = "Rule"
					match = "Host(`vector.${var.domain_main}`)"
					priority = 50
					middlewares = [{
						name = "vector-basic-auth"
						namespace = kubernetes_namespace.vector.0.metadata.0.name
					}]
					services = [{
						name = "vector"
						port = 6200
					}]
				}
			]

			tls = lookup(each.value, "tls", null)
		}
	})
}

resource "kubernetes_secret" "vector_basic_auth_secret" {
	count = var.prometheus_enabled ? 1 : 0
	type = "kubernetes.io/basic-auth"

	metadata {
		name = "vector-route-basic-auth"
		namespace = kubernetes_namespace.vector.0.metadata.0.name
	}

	data = {
		username = module.vector_secrets.values["vector/http/username"]
		password = module.vector_secrets.values["vector/http/password"]
	}
}

# MARK: Middleware
resource "kubectl_manifest" "vector_basic_auth" {
	count = var.prometheus_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "vector-basic-auth"
			namespace = kubernetes_namespace.vector.0.metadata.0.name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			basicAuth = {
				secret = kubernetes_secret.vector_basic_auth_secret.0.metadata.0.name
			}
		}
	})
}