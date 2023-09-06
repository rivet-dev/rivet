locals {
	traffic_server_count = 2
	traffic_server_configmap_data = {
		# TODO:
	}
	traffic_server_configmap_hash = sha256(jsonencode(local.server_configmap_data))
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


# Create a new config map for each version of the config so the stateful set can roll back gracefully.
resource "kubernetes_config_map" "traffic_server" {
	metadata {
		namespace = kubernetes_namespace.traffic_server.metadata.0.name
		name = "traffic-server-configmap"
		labels = {
			app = "traffic-server"
		}
	}

	# TODO:
	data = {}
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
						# TODO: Enable configuration
						storage = "10Gi"
					}
				}
				storage_class_name = "local-path"
			}
		}
	}
}

