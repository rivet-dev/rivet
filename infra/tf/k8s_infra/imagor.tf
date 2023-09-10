locals {
	service_imagor = lookup(var.services, "imagor", {
		count = 1
		resources = {
			cpu = 250
			cpu_cores = 0
			memory = 512
		}
	})

	ephemeral_disk = 8000

	result_storage_s3_endpoint = var.s3_providers[var.s3_default_provider].endpoint_internal
	result_storage_s3_region = var.s3_providers[var.s3_default_provider].region
	result_storage_s3_access_key_id = module.imagor_secrets.values["s3/${var.s3_default_provider}/terraform/key_id"]
	result_storage_s3_secret_access_key = module.imagor_secrets.values["s3/${var.s3_default_provider}/terraform/key"]
	result_storage_s3_bucket = "${var.namespace}-bucket-imagor-result-storage"

	imagor_presets = [
		for preset in var.imagor_presets: {
			key = preset.key
			priority = preset.priority
			game_cors = preset.game_cors

			rule = "(Host(`media.${var.domain_main}`) || HostRegexp(`media.{region:.+}.${var.domain_main}`)) && Path(`${preset.path}`)"
			query = (
				preset.query != null ?
					"&& Query(${join(",", [for x in preset.query: "`${x[0]}=${x[1]}`"])})"
					: ""
			)
			middlewares = (
				preset.game_cors ?
					["imagor-${preset.key}-path", "imagor-cors-game", "imagor-cdn"]
					: ["imagor-${preset.key}-path", "imagor-cors", "imagor-cdn"]
			)
		}
		
	]
}

module "imagor_secrets" {
	source = "../modules/secrets"

	keys = [
		"s3/${var.s3_default_provider}/terraform/key_id",
		"s3/${var.s3_default_provider}/terraform/key",
	]
}

resource "kubernetes_namespace" "imagor" {
	metadata {
		name = "rivet-imagor"
	}
}

resource "kubernetes_priority_class" "imagor_priority" {
	metadata {
		name = "imagor-priority"
	}

	value = 35
}

resource "kubernetes_deployment" "imagor" {
	depends_on = [kubernetes_namespace.imagor]
	
	metadata {
		name = "imagor"
		namespace = "imagor"
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
				priority_class_name = "imagor-priority"
				
				# MARK: Docker auth
				dynamic "image_pull_secrets" {
					for_each = var.authenticate_all_docker_hub_pulls ? toset([1]) : toset([])

					content {
						name = "docker-auth"
					}
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

					resources {
						limits = {
							memory = "${local.service_imagor.resources.memory}Mi"
							cpu = (
								local.service_redis_exporter.resources.cpu_cores > 0 ?
								"${local.service_redis_exporter.resources.cpu_cores * 1000}m"
								: "${local.service_redis_exporter.resources.cpu}m"
							)
							"ephemeral-storage" = "${local.ephemeral_disk}M"
						}
					}
				}
			}
		}
	}
}

resource "kubernetes_secret" "imagor_secret_env" {
	metadata {
		name = "imagor-secret-env"
		namespace = "imagor"
	}
	
	data = {
		"AWS_RESULT_STORAGE_ACCESS_KEY_ID" = base64encode(local.result_storage_s3_access_key_id)
		"AWS_RESULT_STORAGE_SECRET_ACCESS_KEY" = base64encode(local.result_storage_s3_secret_access_key)
	}
}

resource "kubernetes_service" "imagor" {
	depends_on = [kubernetes_namespace.imagor]
	
	metadata {
		name = "imagor"
		namespace = "imagor"
	}
	spec {
		selector = {
			"app.kubernetes.io/name" = kubernetes_deployment.imagor.metadata.0.name
		}

		port {
			protocol = "TCP"
			port = 8000
			target_port = "http"
		}
	}
}

resource "kubectl_manifest" "imagor_traefik_service" {
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "TraefikService"

		metadata = {
			name = "imagor"
			namespace = "imagor"
		}

		spec = {
			mirroring = {
				name = "imagor"
				namespace = "imagor"
				port = 8000
			}
		}
	})
}

# TODO: Create single traefik service
resource "kubectl_manifest" "imagor_ingress" {
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "IngressRoute"

		metadata = {
			name = "imagor"
			namespace = "imagor"
		}

		spec = {
			entryPoints = [ "websecure" ]

			routes = [
				for index, preset in local.imagor_presets:
				{
					kind = "Rule"
					match = "${preset.rule}${preset.query}"
					priority = preset.priority
					middlewares = [
						for mw in preset.middlewares: {
							name = mw
							namespace = "imagor"
						}
					]
					services = [{
						kind = "TraefikService"
						name = "imagor"
						namespace = "imagor"
					}]
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

# MARK: Middleware
resource "kubectl_manifest" "imagor_cors" {
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cors"
			namespace = "imagor"
		}

		spec = {
			headers = {
				accessControlAllowMethods = [ "GET", "OPTIONS" ]
				accessControlAllowOriginList = [ "https://${var.domain_main}" ]
				accessControlMaxAge = 300
			}
		}
	})
}

resource "kubectl_manifest" "imagor_cors_game" {
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cors-game"
			namespace = "imagor"
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
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn-retry"
			namespace = "imagor"
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
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn-cache-control"
			namespace = "imagor"
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
	depends_on = [kubernetes_namespace.imagor, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-cdn"
			namespace = "imagor"
		}

		spec = {
			chain = {
				middlewares = [
					{
						name = "imagor-cdn-retry"
						namespace = "imagor"
					},
					{
						name = "imagor-cdn-cache-control"
						namespace = "imagor"
					}
				]
			}
		}
	})
}

resource "kubectl_manifest" "imagor_preset_middlewares" {
	depends_on = [kubernetes_namespace.traefik, helm_release.traefik]
	for_each = {
		for index, preset in var.imagor_presets:
		preset.key => preset
	}

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = "imagor-${each.key}-path"
			namespace = "imagor"
		}

		spec = {
			replacePathRegex = {
				regex = each.value.path_regexp
				replacement = each.value.path_regex_replacement
			}
		}
	})
}
