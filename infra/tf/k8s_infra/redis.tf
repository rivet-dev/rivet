# TODO: Create multiple of these

resource "kubernetes_namespace" "redis" {
	metadata {
		name = "redis"
	}
}

resource "helm_release" "redis" {
	depends_on = [kubernetes_namespace.redis]

	name = "redis"
	namespace = kubernetes_namespace.redis.metadata.0.name
	repository = "https://charts.bitnami.com/bitnami"
	chart = "redis"
	version = "17.14.6"
	values = [yamlencode({
		replica = {
			replicaCount = 1
		}
		auth = {
			enabled = false
		}

		tls = {
			enabled = true
			authClients = true
			
			existingSecret = "redis-tls-cert"
			certFilename = "tls.crt"
  			certKeyFilename = "tls.key"
  			certCAFilename = "tls.ca"
		}
	})]
}

# Must be created in every namespace it is used in
resource "kubernetes_secret" "redis_tls_cert" {
	depends_on = [kubernetes_namespace.redis, kubernetes_namespace.rivet_service]
	for_each = toset([ "redis", "rivet-service" ])

	metadata {
		name = "redis-tls-cert"
		namespace = each.value
	}

	data = {
		"tls.crt" = data.terraform_remote_state.tls.outputs.test1
		"tls.key" = data.terraform_remote_state.tls.outputs.test2
		"tls.ca" = data.terraform_remote_state.tls.outputs.test3
	}
}
