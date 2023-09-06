locals {
	traffic_server_count = 2
	traffic_server_configmap_data = {
		# TODO:
	}
	traffic_server_configmap_hash = sha256(jsonencode(local.server_configmap_data))
	# TODO: Enable dynamic configuration
	traffic_server_cache_size = 10
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

# TODO: Secrets

resource "kubernetes_config_map" "traffic_server" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server-configmap"
		labels = {
			app = "traffic-server"
		}
	}

	data = merge(
		# Static files
		{
			for f in fileset("${path.module}/files/traffic_server/etc/static", "**/*"):
			f => file("${path.module}/files/traffic_server/etc/${f}/static")
		},
		# Dynamic files
		{
			for f in fileset("${path.module}/files/traffic_server/etc/static", "**/*"):
			f => templatefile("${path.module}/files/traffic_server/etc/${f}/static", {
				s3_providers = var.s3_providers
				volume_size_cache = "${traffic_server_cache_size}G"
			})
		},
		# S3 providers
		# flatten([
		# 	for provider_name, provider in var.s3_providers:
		# 	{
		# 		"s3_region_map_${provider_name}.config" = templatefile("${path.module}/files/traffic_server/s3/s3_region_map.config", {
		# 			s3_endpoint = provider.endpoint_internal
		# 			s3_region = provider.region
		# 		})
		# 	}
		# ]),
	)
}

# TODO: Create secrets
				# "s3_auth_v4_${provider}.config" = templatefile("${path.module}/files/traffic_server/s3/s3_auth_v4.config", {
				# 	s3_access_key_id = TODO
				# 	s3_secret_access_key = TODO
				# 	s3_region_map_file_name = "s3_region_map_${provider}"
				# })

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
		type = "LoadBalancer"
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
				}
			}

			spec {
				security_context {
					run_as_user   = 100
					run_as_group  = 999
					fs_group      = 999
				}
				image_pull_secrets {
					name = kubernetes_secret.traffic_server_docker_config.metadata.0.name
				}
				container {
					name = "traffic-server-instance"
					# TODO: Use the git hash here
					image = "ghcr.io/rivet-gg/apache-traffic-server:1a5aacc"
					image_pull_policy = "IfNotPresent"

					port {
						name = "http"
						container_port = 8080
						protocol = "TCP"
					}

					# TODO:
					# readiness_probe {
						# http_get {
							# path = "/v1/agent/self"
							# port = "http"
						# }
# 
						# initial_delay_seconds = 5
						# period_seconds = 5
					# }

					volume_mount {
						name = "traffic-server-config"
						mount_path = "/etc/trafficserver"
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

