# TODO: Document why we don't use the Consul provider for server discovery like https://www.linode.com/docs/guides/nomad-alongside-kubernetes/
# TODO: Attempt to invalidate this cluster by killing random pods
# TODO: Figure out how to recover from a hard server crash (won't be able to notify other servers of leaving)
# TODO: Scope locals

locals {
	count = 3
	server_service_name = "nomad-server"
	server_addrs = formatlist("nomad-server-statefulset-%d.${local.server_service_name}.${kubernetes_namespace.nomad.metadata.0.name}.svc.cluster.local", range(0, local.count))
	server_addrs_escaped = [for addr in local.server_addrs : "\"${addr}\""]
	server_configmap_data = {
		# We don't use Consul for server discovery because we don't want to depend on Consul for Nomad to work
		"server.hcl" = <<-EOT
			datacenter = "global"
			data_dir = "/opt/nomad/data"
			bind_addr = "0.0.0.0"
			disable_update_check = true

			# The Nomad server IP changes, so we need to leave the cluster when terminating
			leave_on_terminate = true

			server {
				enabled = true
				bootstrap_expect = ${local.count}

				server_join {
					retry_join = [${join(",", local.server_addrs_escaped)}]
					retry_interval = "10s"
				}
			}
		EOT
	}
	server_configmap_hash = sha256(jsonencode(local.server_configmap_data))
}

resource "kubernetes_namespace" "nomad" {
	metadata {
		name = "nomad"
	}
}

# Create a new config map for each version of the config so the stateful set can roll back gracefully.
resource "kubernetes_config_map" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = "nomad-server-configmap-${local.server_configmap_hash}"
		labels = {
			app = "nomad-server"
		}
	}

	data = local.server_configmap_data
}

resource "kubernetes_service" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = local.server_service_name
		labels = {
			name = "nomad-server"
			"app.kubernetes.io/name" = "nomad-server"
		}
	}
	spec {
		selector = {
			app = "nomad-server"
		}
		port {
			name = "http"
			port = 4646
			protocol = "TCP"
		}
		port {
			name = "rpc"
			port = 4647
			protocol = "TCP"
		}
		type = "LoadBalancer"
	}
}

resource "kubernetes_stateful_set" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = "nomad-server-statefulset"
		labels = {
			app = "nomad-server"
		}
	}
	spec {
		replicas = local.count

		selector {
			match_labels = {
				app = "nomad-server"
			}
		}

		service_name = kubernetes_service.nomad_server.metadata.0.name

		template {
			metadata {
				labels = {
					app = "nomad-server"
				}
				annotations = {
					# Trigger a rolling update on config chagne
					"configmap-version" = local.server_configmap_hash
				}
			}

			spec {
				security_context {
					run_as_user   = 100
					run_as_group  = 999
					fs_group      = 999
				}
				container {
					name = "nomad-instance"
					image = "hashicorp/nomad:1.6.0"
					image_pull_policy = "IfNotPresent"
					args = ["agent", "-config=/etc/nomad/nomad.d/server.hcl"]

					port {
						name = "http"
						container_port = 4646
						protocol = "TCP"
					}
					port {
						name = "rpc"
						container_port = 4647
						protocol = "TCP"
					}
					port {
						name = "serf-tcp"
						container_port = 4648
						protocol = "TCP"
					}
					port {
						name = "serf-udp"
						container_port = 4648
						protocol = "UDP"
					}

					# We don't use a readiness probe for `/v1/status/leader` because
					# we need all three nodes to boot successfully and bootstrap.
					# The load balancer itself should prevent routing traffic to
					# nodes that don't have a leader.
					readiness_probe {
						http_get {
							path = "/v1/agent/self"
							port = "http"
						}

						initial_delay_seconds = 5
						period_seconds = 5
					}

					volume_mount {
						name = "nomad-config"
						mount_path = "/etc/nomad/nomad.d"
					}
					volume_mount {
						name = "nomad-data"
						mount_path = "/opt/nomad/data"
					}
				}

				volume {
					name = "nomad-config"
					config_map {
						name = kubernetes_config_map.nomad_server.metadata.0.name
					}
				}
			}
		}

		volume_claim_template {
			metadata {
				name = "nomad-data"
			}
			spec {
				access_modes = ["ReadWriteOnce"]
				resources {
					requests = {
						storage = "1Gi"
					}
				}
				storage_class_name = "local-path"
			}
		}
	}
}

