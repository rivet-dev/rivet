job "nsfw-api:${dc}" {
	type = "service"

	datacenters = ["${dc}"]

	constraint {
		attribute = "$${node.class}"
		value = "${shared.node_class}"
	}

	update {
		max_parallel = 1
		stagger = "1m"
	}

	priority = 40

	group "api" {
		count = ${count}

		restart {
			attempts = 6
			delay = "10s"
			interval = "1m"
			mode = "delay"
		}

		network {
			mode = "bridge"

			${shared.dns_config}

			port "http" {
				static = 21900
			}
		}

		service {
			name = "nsfw-api"
			tags = ["http"]
			port = "http"
			address_mode = "host"
		}

		task "node" {
			driver = "docker"

			config {
				image = "eugencepoi/nsfw_api@sha256:087d880e38b82e5cbee761bafd50e5093a40f813d3f0e77a8077f661cbcdb414"
				${shared.docker_auth}

				ports = ["http"]
			}

			env {
				PORT = "$${NOMAD_PORT_http}"
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
}
