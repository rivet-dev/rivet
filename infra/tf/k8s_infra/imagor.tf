locals {
	service_imagor = lookup(var.services, "imagor", {
		count = var.deploy_method_cluster ? 2 : 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})

	result_storage_s3_endpoint = var.s3.endpoint_internal
	result_storage_s3_region = var.s3.region
	result_storage_s3_access_key_id = module.imagor_secrets.values["s3/terraform/key_id"]
	result_storage_s3_secret_access_key = module.imagor_secrets.values["s3/terraform/key"]
	result_storage_s3_bucket = "${var.namespace}-bucket-imagor-result-storage"

	imagor_presets = flatten([
		for preset in var.imagor_presets:
		{
			key = preset.key
			priority = preset.priority
			game_cors = preset.game_cors

			match = "Path(`/media${preset.path}`)${
				var.domain_main_api != null ?
					"&& Host(`${var.domain_main_api}`)" :
					""
			}${
				preset.query != null ?
					" && Query(${join(",", [for x in preset.query: "`${x[0]}=${x[1]}`"])})" :
					""
			}"

			middlewares = (
				preset.game_cors ?
					["imagor-${preset.key}-path", "imagor-cors-game", "imagor-cdn"]
					: ["imagor-${preset.key}-path", "imagor-cors", "imagor-cdn"]
			)
		}
	])
}

module "imagor_secrets" {
	source = "../modules/secrets"

	keys = [
		"s3/terraform/key_id",
		"s3/terraform/key",
	]
}

resource "kubernetes_namespace" "imagor" {
	count = var.imagor_enabled ? 1 : 0

	metadata {
		name = "imagor"
	}
}

resource "kubernetes_deployment" "imagor" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [null_resource.daemons, module.docker_auth]

	metadata {
		name = "imagor"
		namespace = kubernetes_namespace.imagor.0.metadata[0].name
	}

	spec {
		replicas = local.service_imagor.count

		selector {
			match_labels = {
				"app.kubernetes.io/name" = "imagor"
			}
		}

		template {
			metadata {
				labels = {
					"app.kubernetes.io/name" = "imagor"
				}
			}

			spec {
				priority_class_name = kubernetes_priority_class.service_priority.metadata.0.name
				
				# MARK: Docker auth
				image_pull_secrets {
					name = "docker-auth"
				}

				container {
					image = "shumc/imagor:1.4.7"
					name = "imagor"

					env {
					  name = "PORT"
					  value = 8000
					}
					# Unsafe is fine since we don't expose Imagor publicly and use explicit
					# rules Traefik to filter what requests can be made.
					env {
						name = "IMAGOR_UNSAFE"
						value = "1"
					}
					env {
						name = "S3_RESULT_STORAGE_ENDPOINT"
						value = "${local.result_storage_s3_endpoint}"
					}
					env {
						name = "AWS_RESULT_STORAGE_REGION"
						value = "${local.result_storage_s3_region}"
					}
					env {
						name = "S3_RESULT_STORAGE_BUCKET"
						value = "${local.result_storage_s3_bucket}"
					}
					env {
						name = "S3_RESULT_STORAGE_EXPIRATION"
						value = "48h"
					}
					env {
						name = "S3_FORCE_PATH_STYLE"
						value = "1"
					}
					env_from {
						secret_ref {
							name = "imagor-secret-env"
						}
					}

					port {
						name = "http"
						container_port = 8000
					}
					
					liveness_probe {
						http_get {
							path = "/"
							port = 8000
						}
						
						initial_delay_seconds = 1
						period_seconds = 5
						timeout_seconds = 2
					}

					dynamic "resources" {
						for_each = var.limit_resources ? [0] : []

						content {
							limits = {
								memory = "${local.service_imagor.resources.memory}Mi"
								cpu = "${local.service_imagor.resources.cpu}m"
								"ephemeral-storage" = "8Gi"
							}
						}
					}
				}
			}
		}
	}
}

resource "kubernetes_secret" "imagor_secret_env" {
	count = var.imagor_enabled ? 1 : 0

	metadata {
		name = "imagor-secret-env"
		namespace = kubernetes_namespace.imagor.0.metadata[0].name
	}
	
	data = {
		"AWS_RESULT_STORAGE_ACCESS_KEY_ID" = base64encode(local.result_storage_s3_access_key_id)
		"AWS_RESULT_STORAGE_SECRET_ACCESS_KEY" = base64encode(local.result_storage_s3_secret_access_key)
	}
}

resource "kubernetes_service" "imagor" {
	count = var.imagor_enabled ? 1 : 0

	metadata {
		name = "imagor"
		namespace = kubernetes_namespace.imagor.0.metadata[0].name
	}
	spec {
		selector = {
			"app.kubernetes.io/name" = kubernetes_deployment.imagor.0.metadata.0.name
		}

		port {
			protocol = "TCP"
			port = 8000
			target_port = "http"
		}
	}
}

resource "kubectl_manifest" "imagor_traefik_service" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "TraefikService"

		metadata = {
			name = "imagor"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			mirroring = {
				name = "imagor"
				namespace = kubernetes_namespace.imagor.0.metadata[0].name
				port = 8000
			}
		}
	})
}

resource "kubectl_manifest" "imagor_ingress" {
	for_each = var.imagor_enabled ? local.entrypoints : {}

	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "IngressRoute"

		metadata = {
			name = "imagor-${each.key}"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			entryPoints = [ each.key ]

			routes = [
				for index, preset in local.imagor_presets:
				{
					kind = "Rule"
					match = preset.match
					priority = preset.priority
					middlewares = [
						for mw in preset.middlewares: {
							name = mw
							namespace = kubernetes_namespace.imagor.0.metadata[0].name
						}
					]
					services = [{
						kind = "TraefikService"
						name = "imagor"
						namespace = kubernetes_namespace.imagor.0.metadata[0].name
					}]
				}
			]

			tls = lookup(each.value, "tls", null)
		}
	})
}

# MARK: Middleware
resource "kubectl_manifest" "imagor_cors" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cors"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			headers = {
				accessControlAllowMethods = [ "GET", "OPTIONS" ]
				accessControlAllowOriginList = var.imagor_cors_allowed_origins
				accessControlMaxAge = 300
			}
		}
	})
}

resource "kubectl_manifest" "imagor_cors_game" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cors-game"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			headers = {
				accessControlAllowMethods = [ "GET", "OPTIONS" ]
				accessControlAllowOriginList = [ "*" ]
				accessControlMaxAge = 300
			}
		}
	})
}

resource "kubectl_manifest" "imagor_cdn_retry" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn-retry"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			retry = {
				attempts = 4
				initialInterval = "1s"
			}
		}
	})
}

resource "kubectl_manifest" "imagor_cdn_cache_control" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn-cache-control"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			headers = {
				customResponseHeaders = {
					"Cache-Control" = "public, max-age=604800, immutable"
				}
			}
		}
	})
}

resource "kubectl_manifest" "imagor_cdn" {
	count = var.imagor_enabled ? 1 : 0
	depends_on = [helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			chain = {
				middlewares = [
					{
						name = "imagor-cdn-retry"
						namespace = kubernetes_namespace.imagor.0.metadata[0].name
					},
					{
						name = "imagor-cdn-cache-control"
						namespace = kubernetes_namespace.imagor.0.metadata[0].name
					}
				]
			}
		}
	})
}

resource "kubectl_manifest" "imagor_preset_middlewares" {
	depends_on = [helm_release.traefik]
	for_each = {
		for index, preset in var.imagor_presets:
		preset.key => preset
		if var.imagor_enabled
	}

	yaml_body = yamlencode({
		apiVersion = "traefik.io/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-${each.key}-path"
			namespace = kubernetes_namespace.imagor.0.metadata[0].name
			labels = {
				"traefik-instance" = "main"
			}
		}

		spec = {
			replacePathRegex = {
				regex = each.value.path_regexp
				replacement = each.value.path_regex_replacement
			}
		}
	})
}
