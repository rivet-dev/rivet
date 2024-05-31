locals {
	service_vector = lookup(var.services, "vector", {
		count = 1
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
	version = "0.29.0"
	values = [yamlencode({
		role = "Aggregator"
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
				address = "127.0.0.1:8686"
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

				clickhouse_cf_logs = {
					type = "clickhouse"
					inputs = ["http_json"]
					compression = "gzip"
					database = "db_cf_log"
					endpoint = "https://${var.clickhouse_host}:${var.clickhouse_port_https}"
					table = "cf_tail_events"
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
						timeout_secs = 5.0
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

