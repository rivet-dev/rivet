# Run the Nomad servers used to orchestrate jobs on the Nomad game servers.
#
# Servers talk to each (i.e. Nomad Serf) to each other via a sidecar proxy (i.e. TCP + UDP) which has a consistent
# IP + port. We do this because each server needs to have a consistent IP address it can be access at so nodes know
# how to find eachother after restarting. If the IP address changed (as it would if using CoreDNS with the pod's IP),
# then the cluster would fail to bootstrap after starting at the new pod's IP address.
#
# If we used CoreDNS instsead (i.e. directly connect to `nomad-server-statefulset-${i}.nomad-server.nomad.svc.cluster.local`),
# the servers would resolve the IP address from the DNS record and store that. When moving the server to a new pod's IP, the server
# would fail to connect to the other servers because their IP changed.
#
# We don't use Consul for Nomad server service discvoery because (a) we'd have to _also_ run a Consul cluster which is unnecessarily
# complicated + adds another point of failure and (b) it doesn't fix the problem with Nomad server addresses changing.

locals {
	# !!! DO NOT CHANGE !!!
	#
	# This value must be 3, 5, or 7. More = better redundancy, but does not make things faster.
	# 
	# See https://developer.hashicorp.com/nomad/tutorials/enterprise/production-reference-architecture-vm-with-consul
	nomad_server_count = 3

	nomad_server_addrs = [for i in range(0, local.nomad_server_count): "127.0.0.1:${6000 + i}"]
	nomad_server_addrs_escaped = [for addr in local.nomad_server_addrs : "\"${addr}\""]
	nomad_server_configmap_data = {
		"server.hcl" = <<-EOT
			datacenter = "global"
			data_dir = "/opt/nomad/data"
			bind_addr = "0.0.0.0"
			disable_update_check = true

			advertise {
				rpc = "127.0.0.1:__LOCAL_PORT_RPC__"
				serf = "127.0.0.1:__LOCAL_PORT_SERF__"
			}

			server {
				enabled = true
				bootstrap_expect = ${local.nomad_server_count}

				server_join {
					retry_join = [${join(", ", local.nomad_server_addrs_escaped)}]
					retry_interval = "10s"
				}
			}

			telemetry {
				prometheus_metrics = true
			}

			limits {
				# Increase max connections for Prometheus monitors
				http_max_conns_per_client = 4096

				# All Nomad clients come from the same IP address, so we need to
				# disable the max RPC connections per IP
				rpc_max_conns_per_client = 0
			}
		EOT
	}
	nomad_checksum_configmap = sha256(jsonencode(local.nomad_server_configmap_data))

	# Recommendations: https://developer.hashicorp.com/nomad/docs/install/production/requirements#resources-ram-cpu-etc
	service_nomad = lookup(var.services, "nomad", {
		count = 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})
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
		name = "nomad-server-configmap-${local.nomad_checksum_configmap}"
		labels = {
			app = "nomad-server"
		}
	}

	data = local.nomad_server_configmap_data
}

# Expose service
resource "kubernetes_service" "nomad_server" {
	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = "nomad-server"
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
	}
}

resource "kubectl_manifest" "nomad_server_monitor" {
	depends_on = [kubernetes_stateful_set.nomad_server]

	yaml_body = yamlencode({
		apiVersion = "monitoring.coreos.com/v1"
		kind = "ServiceMonitor"

		metadata = {
			name = "nomad-server-service-monitor"
			namespace = kubernetes_namespace.nomad.metadata.0.name
		}

		spec = {
			selector = {
				matchLabels = {
					"app.kubernetes.io/name": "nomad-server"
				}
			}
			endpoints = [
				{
					port = "http"
					path = "/v1/metrics"
					params = {
						format = ["prometheus"]
					}
				}
			]
		}
	})
}

resource "kubernetes_priority_class" "nomad_priority" {
	metadata {
		name = "nomad-priority"
	}

	value = 40
}

resource "kubernetes_stateful_set" "nomad_server" {
	depends_on = [null_resource.daemons]

	metadata {
		namespace = kubernetes_namespace.nomad.metadata.0.name
		name = "nomad-server-statefulset"
		labels = {
			app = "nomad-server"
		}
	}
	spec {
		replicas = local.nomad_server_count

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
					# Trigger a rolling update on config change
					"checksum/configmap" = local.nomad_checksum_configmap
				}
			}

			spec {
				priority_class_name = kubernetes_priority_class.nomad_priority.metadata.0.name

				security_context {
					run_as_user = 0
				}

				container {
					name = "nomad-instance"
					# IMPORTANT: Do not upgrade past 1.6.0. This is the last MPL-licensed version.
					image = "hashicorp/nomad:1.6.0"
					image_pull_policy = "IfNotPresent"

					command = ["/bin/sh", "-c"]
					args = [
						<<-EOF
						# Calculate the local ports
						POD_INDEX=$(echo $POD_NAME | awk -F'-' '{print $NF}')
						LOCAL_PORT_RPC=$((5000 + POD_INDEX))
						LOCAL_PORT_SERF=$((6000 + POD_INDEX))
						echo "Pod index: $POD_INDEX"
						echo "Port RPC: $LOCAL_PORT_RPC"
						echo "Port Serf: $LOCAL_PORT_SERF"

						# Template the config
						mkdir -p /etc/nomad/nomad.d/
						cat /tpl/etc/nomad/nomad.d/server.hcl | sed -e "s/__LOCAL_PORT_RPC__/$${LOCAL_PORT_RPC}/g; s/__LOCAL_PORT_SERF__/$${LOCAL_PORT_SERF}/g"  > /etc/nomad/nomad.d/server.hcl

						# Start agent
						nomad agent -config=/etc/nomad/nomad.d/server.hcl
						EOF
					]

					env {
						name = "POD_NAME"
						value_from {
							field_ref {
								field_path = "metadata.name"
							}
						}
					}

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
						mount_path = "/tpl/etc/nomad/nomad.d"
					}

					volume_mount {
						name = "nomad-data"
						mount_path = "/opt/nomad/data"
					}
				}

				# Sidecar that proxies traffic to other Nomad clients.
				container {
					name  = "traefik-sidecar"
					image = "traefik:v2.10"
					args = flatten([
						# Generic config
						[
							"--providers.file.directory=/etc/traefik",
							"--providers.file.watch=true"
						],

						# Entrypoints
						flatten([
							for i in range(0, local.nomad_server_count):
							[
								"--entryPoints.nomad-${i}-rpc-tcp.address=:${5000 + i}/tcp",
								"--entryPoints.nomad-${i}-serf-tcp.address=:${6000 + i}/tcp",
								"--entryPoints.nomad-${i}-serf-udp.address=:${6000 + i}/udp",
							]
						]),
					])

					dynamic "port" {
						for_each = [for i in range(0, local.nomad_server_count) : i]
						content {
							name = "n-${port.value}-rpc-tcp"
							container_port = 5000 + port.value
							protocol = "TCP"
						}
					}

					dynamic "port" {
						for_each = [for i in range(0, local.nomad_server_count) : i]
						content {
							name = "n-${port.value}-serf-tcp"
							container_port = 6000 + port.value
							protocol = "TCP"
						}
					}

					dynamic "port" {
						for_each = [for i in range(0, local.nomad_server_count) : i]
						content {
							name = "n-${port.value}-serf-udp"
							container_port = 6000 + port.value
							protocol = "UDP"
						}
					}

					volume_mount {
						name       = "traefik-config"
						mount_path = "/etc/traefik"
					}

					dynamic "resources" {
						for_each = var.limit_resources ? [0] : []

						content {
							limits = {
								cpu = "${local.service_nomad.resources.cpu}m"
								memory = "${local.service_nomad.resources.memory}Mi"
							}
						}
					}
				}

				volume {
					name = "nomad-config"
					config_map {
						name = kubernetes_config_map.nomad_server.metadata.0.name
					}
				}

				volume {
					name = "traefik-config"
					config_map {
						name = kubernetes_config_map.nomad_server_sidecar_traefik_config.metadata[0].name
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
						storage = "64Gi"
					}
				}
				storage_class_name = var.k8s_storage_class
			}
		}
	}
}

# Build Traefik config for the sidecar that forwards traffic to other Nomad leaders.
resource "kubernetes_config_map" "nomad_server_sidecar_traefik_config" {
	metadata {
		name = "nomad-server-sidecar-traefik"
		namespace = kubernetes_namespace.nomad.metadata[0].name
	}

	data = {
		for i in range(0, local.nomad_server_count):
		"nomad-${i}.yaml" => yamlencode({
			tcp = {
				routers = {
					"nomad-${i}-rpc" = {
						entryPoints = ["nomad-${i}-rpc-tcp"]
						rule = "HostSNI(`*`)"
						service = "nomad-${i}-rpc"
					}
					"nomad-${i}-serf" = {
						entryPoints = ["nomad-${i}-serf-tcp"]
						rule = "HostSNI(`*`)"
						service = "nomad-${i}-serf"
					}
				}
				services = {
					"nomad-${i}-rpc" = {
						loadBalancer = {
							servers = [
								{
									address = "nomad-server-statefulset-${i}.nomad-server.nomad.svc.cluster.local:4647"
								}
							]
						}
					}
					"nomad-${i}-serf" = {
						loadBalancer = {
							servers = [
								{
									address = "nomad-server-statefulset-${i}.nomad-server.nomad.svc.cluster.local:4648"
								}
							]
						}
					}
				}
			}
			udp = {
				routers = {
					"nomad-${i}-serf" = {
						entryPoints = ["nomad-${i}-serf-udp"]
						service = "nomad-${i}-serf"
					}
				}
				services = {
					"nomad-${i}-serf" = {
						loadBalancer = {
							servers = [
								{
									address = "nomad-server-statefulset-${i}.nomad-server.nomad.svc.cluster.local:4648"
								}
							]
						}
					}
				}
			}
		})
	}
}
