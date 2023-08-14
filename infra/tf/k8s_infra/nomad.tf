# TODO: Document why we don't use the Consul provider for server discovery like https://www.linode.com/docs/guides/nomad-alongside-kubernetes/
# TODO: Look at using Reloader  (https://github.com/stakater/Reloader) for graceful migrations, but this adds extra complexity
# TODO: Attempt to kill this cluster
# TODO: Figure out health checks

locals {
	count = 3
	server_service_name = "nomad-server"
	server_addrs = formatlist("nomad-server-statefulset-%d.${local.server_service_name}.${kubernetes_namespace.nomad.metadata.0.name}.svc.cluster.local", range(0, local.count))
	server_addrs_escaped = [for addr in local.server_addrs : "\"${addr}\""]
}

resource "kubernetes_namespace" "nomad" {
	metadata {
		name = "nomad"
	}
}

resource "kubernetes_config_map" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = "nomad-server-configmap"
		labels = {
			app = "nomad-server"
		}
	}

	data = {
		"server.hcl" = <<-EOT
			datacenter = "global"
			data_dir = "/opt/nomad/data"
			bind_addr = "0.0.0.0"
			disable_update_check = true

			server {
				enabled = true
				bootstrap_expect = ${local.count}
				retry_join = [${join(",", local.server_addrs_escaped)}]
				retry_interval = "10s"
			}
		EOT
	}
}

resource "kubernetes_service" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = local.server_service_name
		labels = {
			name = "nomad-server"
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

				volume {
					name = "nomad-data"
					empty_dir {}
				}
			}
		}
	}
}

