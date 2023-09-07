# TODO: Add back body_factory

locals {
	traffic_server_count = 2
	traffic_server_configmap_data = merge(
		# Static files
		{
			for f in fileset("${path.module}/files/traffic_server/etc/static", "**/*"):
			f => file("${path.module}/files/traffic_server/etc/static/${f}")
		},
		# Dynamic files
		{
			for f in fileset("${path.module}/files/traffic_server/etc/dynamic", "**/*"):
			f => templatefile("${path.module}/files/traffic_server/etc/dynamic/${f}", {
				s3_providers = var.s3_providers
				s3_default_provider = var.s3_default_provider
				volume_size_cache = "${local.traffic_server_cache_size}G"
			})
		},
		# S3 providers
		flatten([
			for provider_name, provider in var.s3_providers:
			{
				"s3_region_map_${provider_name}.config" = templatefile("${path.module}/files/traffic_server/etc/s3/s3_region_map.config", {
					s3_host = element(regex("^(?:https?://)?([^/]+)", provider.endpoint_internal), 1)
					s3_region = provider.region
				})
			}
		])...
	)

	traffic_server_s3_auth_data = {
		for provider_name, provider in var.s3_providers:
		"s3_auth_v4_${provider_name}.config" => templatefile("${path.module}/files/traffic_server/etc/s3/s3_auth_v4.config", {
			s3_provider_name = provider_name
			s3_access_key_id = module.traffic_server_s3_secrets.values["s3/${provider_name}/terraform/key_id"]
			s3_secret_access_key = module.traffic_server_s3_secrets.values["s3/${provider_name}/terraform/key"]
		})
	}

	traffic_server_configmap_hash = sha256(jsonencode(local.traffic_server_configmap_data))
	traffic_server_s3_auth_hash = sha256(jsonencode(local.traffic_server_s3_auth_data))

	# TODO: Enable dynamic configuration
	traffic_server_cache_size = 10
}

module "traffic_server_s3_secrets" {
	source = "../modules/secrets"

	keys = flatten([
		for provider_name, provider in var.s3_providers:
		["s3/${provider_name}/terraform/key_id", "s3/${provider_name}/terraform/key"]
	])
}

resource "kubernetes_namespace" "traffic_server" {
	metadata {
		name = "traffic-server"
	}
}

resource "kubernetes_secret" "traffic_server_docker_config" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "ghcr-registry-credentials"
	}

	data = local.ghcr_registry_data

	type = "kubernetes.io/dockerconfigjson"

}

resource "kubernetes_config_map" "traffic_server" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server-configmap"
		labels = {
			app = "traffic-server"
		}
	}

	data = local.traffic_server_configmap_data
}

resource "kubernetes_secret" "traffic_server_s3_auth" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server-s3-auth"
	}

	data = local.traffic_server_s3_auth_data
}

resource "kubernetes_service" "traffic_server" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server"
		labels = {
			name = "traffic-server"
			"app.kubernetes.io/name" = "traffic-server"
		}
	}
	spec {
		selector = {
			app = "traffic-server"
		}
		port {
			name = "http"
			port = 8080
			protocol = "TCP"
		}
	}
}

resource "kubernetes_stateful_set" "traffic_server" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server-statefulset"
		labels = {
			app = "traffic-server"
		}
	}
	spec {
		replicas = local.traffic_server_count

		selector {
			match_labels = {
				app = "traffic-server"
			}
		}

		service_name = kubernetes_service.traffic_server.metadata.0.name

		template {
			metadata {
				labels = {
					app = "traffic-server"
				}
				annotations = {
					# Trigger a rolling update on config chagne
					"configmap-version" = local.traffic_server_configmap_hash
					"s3-auth-version" = local.traffic_server_s3_auth_hash
				}
			}

			spec {
				image_pull_secrets {
					name = kubernetes_secret.traffic_server_docker_config.metadata.0.name
				}
				container {
					name = "traffic-server-instance"
					# TODO: Use the git hash here
					image = "ghcr.io/rivet-gg/apache-traffic-server:5e1d6e5"
					image_pull_policy = "IfNotPresent"

					port {
						name = "http"
						container_port = 8080
						protocol = "TCP"
					}

					liveness_probe {
						tcp_socket {
							port = 8080
						}
						
						initial_delay_seconds = 1
						period_seconds = 5
						timeout_seconds = 2
					}

					volume_mount {
						name = "traffic-server-config"
						mount_path = "/etc/trafficserver"
					}
					volume_mount {
						name = "traffic-server-s3-auth"
						mount_path = "/etc/trafficserver-s3-auth"
					}
					volume_mount {
						name = "traffic-server-cache"
						mount_path = "/var/cache/trafficserver"
					}
				}

				volume {
					name = "traffic-server-config"
					config_map {
						name = kubernetes_config_map.traffic_server.metadata.0.name
					}
				}

				volume {
					name = "traffic-server-s3-auth"
					secret {
						secret_name = kubernetes_secret.traffic_server_s3_auth.metadata.0.name
					}
				}
			}
		}

		volume_claim_template {
			metadata {
				name = "traffic-server-cache"
			}
			spec {
				access_modes = ["ReadWriteOnce"]
				resources {
					requests = {
						storage = "${local.traffic_server_cache_size}Gi"
					}
				}
				storage_class_name = "local-path"
			}
		}
	}
}

resource "kubectl_manifest" "traffic_server_traefik_service" {
	depends_on = [kubernetes_namespace.traffic_server, helm_release.traefik]

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "TraefikService"

		metadata = {
			name = "traffic-server"
			namespace = "traffic-server"
		}

		spec = {
			mirroring = {
				name = "traffic-server"
				namespace = "traffic-server"
				port = 8080
			}
		}
	})
}

locals {
	traffic_server_middlewares = {
		"traffic-server-cors" = {
			headers = {
				accessControlAllowMethods = [ "GET", "OPTIONS" ]
				accessControlAllowOriginList = [ "https://${var.domain_main}" ]
				accessControlMaxAge = 300
			}
		}
		"traffic-server-cors-game" = {
			headers = {
				accessControlAllowMethods = [ "GET", "OPTIONS" ]
				accessControlAllowOriginList = [ "*" ]
				accessControlMaxAge = 300
			}
		}
		"traffic-server-cdn" = {
			chain = {
				middlewares = [
					for x in ["traffic-server-cdn-retry", "traffic-server-cdn-compress", "traffic-server-cdn-cache-control"]:
					{ name = x, namespace = "traffic-server" }
				]
			}
		}
		"traffic-server-cdn-retry" = {
			retry = {
				attempts = 2
				initialInterval = "1s"
			}
		}
		"traffic-server-cdn-compress" = {
			compress = { compress = true }
		}
		"traffic-server-cdn-cache-control" = {
			headers = {
				customResponseHeaders = {
					# See https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#caching_static_assets
					# and https://imagekit.io/blog/ultimate-guide-to-http-caching-for-static-assets/
					"Cache-Control" = "public, max-age=604800, immutable"
				}
			}
		}
	}
}

resource "kubectl_manifest" "traffic_server_middlewares" {
	depends_on = [kubernetes_namespace.traffic_server, helm_release.traefik]

	for_each = local.traffic_server_middlewares

	yaml_body = yamlencode({
		apiVersion = "traefik.containo.us/v1alpha1"
		kind = "Middleware"
		
		metadata = {
			name = each.key
			namespace = "traffic-server"
		}

		spec = each.value
	})
}

