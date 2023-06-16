job "consul-exporter:${dc}" {
	type = "service"

	datacenters = ["${dc}"]

	constraint {
		attribute = "$${node.class}"
		value = "${shared.node_class}"
	}

	priority = 40

	group "consul_exporter" {
		count = 1

		network {
			mode = "bridge"

			${shared.dns_config}

			port "http" {
				static = "21010"
			}
		}

		service {
			name = "consul-exporter"
			tags = ["http"]
			port = "http"
			address_mode = "host"
		}

		task "consul_exporter" {
			driver = "docker"

			config {
				image = "prom/consul-exporter:v0.8.0"
				${shared.docker_auth}
				args = ["--consul.server=$${attr.unique.network.ip-address}:8500", "--web.listen-address=0.0.0.0:$${NOMAD_PORT_http}"]

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
}
