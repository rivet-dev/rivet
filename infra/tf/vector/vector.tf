locals {
	service_vector = lookup(var.services, "vector", {
		count = var.deploy_method_cluster ? 2 : 1
		resources = {
			cpu = 1000
			memory = 2048
		}
	})

	clickhouse_k8s = var.clickhouse_enabled && var.clickhouse_provider == "kubernetes"
}

module "secrets" {
	source = "../modules/secrets"

	keys = [
		"clickhouse/users/vector/password"
	]
}

resource "helm_release" "vector" {
	name = "vector"
	namespace = "vector"
	repository = "https://helm.vector.dev"
	chart = "vector"
	version = "0.33.0"
	values = [yamlencode({
		role = "Aggregator"
		replicas = local.service_vector.count
		podPriorityClassName = "service-priority"
		resources = var.limit_resources ? {
			limits = {
				memory = "${local.service_vector.resources.memory}Mi"
				cpu = "${local.service_vector.resources.cpu}m"
			}
		} : null
		podMonitor = {
			enabled = true
		}
		customConfig = {
			data_dir = "/vector-data-dir"
			api = {
				enabled = true
				address = "0.0.0.0:8686"
				playground = false
			}
			sources = {
				vector = {
					type = "vector"
					address = "0.0.0.0:6000"
				}

				tcp_json = {
					type = "socket"
					mode = "tcp"
					address = "0.0.0.0:6100"
					decoding = { codec = "json" }
				}

				http_json = {
					type = "http_server"
					address = "0.0.0.0:6200"
					decoding = { codec = "json" }
					path = ""
					strict_path = false
				}
				
				vector_metrics = {
					type = "internal_metrics"
				}
				vector_logs = {
					type = "internal_logs"
				}
			}
			transforms = {
				job_run = {
					type = "filter"
					inputs = ["vector", "tcp_json"]
					condition = {
						type = "vrl"
						source = ".source == \"job_run\""
					}
				}

				dynamic_servers = {
					type = "filter"
					inputs = ["vector", "tcp_json"]
					condition = {
						type = "vrl"
						source = ".source == \"dynamic_servers\""
					}
				}

				ds_fix_id = {
					type = "remap"
					inputs = ["dynamic_servers"]
					source = <<-EOF
						.server_id = .run_id
						del(.run_id)
					EOF
				}

				backend_worker = {
					type = "filter"
					inputs = ["http_json"]
					condition = {
						type = "vrl"
						source = ".path == \"/backend\""
					}
				}
			}
			sinks = {
				prom_exporter = {
					type = "prometheus_exporter"
					inputs = ["vector", "vector_metrics"]
					address = "0.0.0.0:9598"
				}

				clickhouse_job_run_logs = {
					type = "clickhouse"
					inputs = ["job_run"]
					compression = "gzip"
					database = "db_job_log"
					endpoint = "https://${var.clickhouse_host}:${var.clickhouse_port_https}"
					table = "run_logs"
					auth = {
						strategy = "basic"
						user = "vector"
						# Escape values for Vector
						password = replace(module.secrets.values["clickhouse/users/vector/password"], "$", "$$")
					}
					tls = local.clickhouse_k8s ? {
						ca_file = "/usr/local/share/ca-certificates/clickhouse-ca.crt"
					} : {}
					batch = {
						# Speed up for realtime-ish logs
						timeout_secs = 1.0
					}
				}

				clickhouse_ds_logs = {
					type = "clickhouse"
					inputs = ["ds_fix_id"]
					compression = "gzip"
					database = "db_ds_log"
					endpoint = "https://${var.clickhouse_host}:${var.clickhouse_port_https}"
					table = "server_logs"
					auth = {
						strategy = "basic"
						user = "vector"
						# Escape values for Vector
						password = replace(module.secrets.values["clickhouse/users/vector/password"], "$", "$$")
					}
					tls = local.clickhouse_k8s ? {
						ca_file = "/usr/local/share/ca-certificates/clickhouse-ca.crt"
					} : {}
					batch = {
						# Speed up for realtime-ish logs
						timeout_secs = 1.0
					}
				}

				clickhouse_backend_logs = {
					type = "clickhouse"
					inputs = ["backend_worker"]
					compression = "gzip"
					database = "db_cf_log"
					endpoint = "https://${var.clickhouse_host}:${var.clickhouse_port_https}"
					table = "tail_events"
					auth = {
						strategy = "basic"
						user = "vector"
						# Escape values for Vector
						password = replace(module.secrets.values["clickhouse/users/vector/password"], "$", "$$")
					}
					tls = local.clickhouse_k8s ? {
						ca_file = "/usr/local/share/ca-certificates/clickhouse-ca.crt"
					} : {}
					batch = {
						# Speed up for realtime-ish logs
						timeout_secs = 1.0
					}
				}

				console = {
					type = "console"
					inputs = ["vector_logs"]
					encoding = {
						codec = "text"
					}
				}
			}
		}
		extraVolumes = local.clickhouse_k8s ? [
			{
				name = "clickhouse-ca",
				configMap = {
					name = "clickhouse-ca",
					defaultMode = 420,
					items = [
						{
							key = "ca.crt",
							path = "clickhouse-ca.crt"
						}
					]
				}
			}
		] : []
		extraVolumeMounts = local.clickhouse_k8s ? [
			{
				name = "clickhouse-ca",
				mountPath = "/usr/local/share/ca-certificates/clickhouse-ca.crt",
				subPath = "clickhouse-ca.crt"
			}
		] : []

		# env = [
		# 	{
		# 		name = "VECTOR_LOG"
		# 		value = "debug"
		# 	}
		# ]
	})]
}

