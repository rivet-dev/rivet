job "redis-exporter:${dc}" {
	type = "service"

	datacenters = ["${dc}"]

	constraint {
		attribute = "$${node.class}"
		value = "${shared.node_class}"
	}

	priority = 40

	%{ for service_id, service in redis_svcs }
	group "redis_exporter_${replace(service_id, "-", "_")}" {
		count = 1

		network {
			mode = "bridge"

			${shared.dns_config}

			port "http" {
				to = 9121
			}
		}

		service {
			name = "redis-exporter-${service_id}"
			tags = [
				"http",
				"redis-exporter",
			]
			port = "http"
			address_mode = "host"
		}

		task "redis_exporter" {
			driver = "docker"

			config {
				image = "oliver006/redis_exporter:v1.33.0"
				${shared.docker_auth}
				args = [
					"--redis.addr=${service.endpoint}",
					"--redis.user=${service.username}",
					"--redis.password=${service.password}",
					%{ if service_id == "redis-chirp" }
					"--check-streams=chirp:topic:*",
					%{ endif }
				]

				ports = ["http"]
			}

			resources {
				%{ if resources.cpu_cores > 0 }
					cores = ${resources.cpu_cores}
				%{ else }
					cpu = ${resources.cpu}
				%{ endif }
				memory = ${resources.memory}
			}

			logs {
				max_files = 4
				max_file_size = 4
			}
		}
	}
	%{ endfor }
}
