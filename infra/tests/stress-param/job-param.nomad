
job "test-param" {
	region = "do-sfo"
	datacenters = ["do-sfo3"]

	type = "batch"

	parameterized {
		payload = "forbidden"
		meta_required = ["X"]
		meta_optional = []
	}

	constraint {
		attribute = "${node.class}"
		value = "job"
	}

	group "test" {
		count = 1

		network {
			mode = "bridge"

			port "test" {
				to = 1234
			}
		}

		service {
			name = "test-param-test"
			port = "test"
			address_mode = "host"

			# TODO: Health check
			#
		}

		task "test" {
			driver = "docker"

			config {
				image = "alpine:3.14"
				entrypoint = ["/bin/sh", "-euf", "/local/run.sh"]
			}

			artifact {
				source = "https://assets.rivet.gg/graphics/landing.png"
				destination = "local/landing.png"
			}

			template {
				destination = "local/run.sh"
				perms = "544"
				change_mode = "restart"
				data = <<-EOF
				echo 'Starting {{ env "NOMAD_META_X" }}...'

				echo "nats"
				{{ range service "listen.nats" }}
					echo "* {{ .Address }}:{{ .Port }}"
				{{ end }}

				echo "crdb-listen"
				{{ range datacenters }}
					{{ $dc := . }}
					echo "* {{$dc}}"
					{{ range service (print "listen.crdb@" $dc "|any") }}
						echo "  * {{index .NodeMeta "network-public-ipv4"}}:{{.Port}}"
					{{ end }}
				{{ end }}

				# Turn this up in order to ensure all jobs are running in paralllel
				sleep 5
				echo 'Finished'
				EOF
			}

			# template {
			# 	destination = "local/run.sh"
			# 	perms = "544"
			# 	change_mode = "restart"
			# 	data = <<-EOF
			# 	echo 'Starting {{ env "NOMAD_META_X" }}...'

			# 	sleep 5
			# 	echo 'Finished'
			# 	EOF
			# }

			resources {
				cpu = 10
				memory = 50
			}

			logs {
				max_files = 4
				max_file_size = 4
			}
		}

		# task "prestart-test" {
		# 	lifecycle {
		# 		hook = "prestart"
		# 		sidecar = false
		# 	}

		# 	driver = "docker"

		# 	config {
		# 		image = "alpine:3.14"
		# 		entrypoint = ["/bin/sh", "-euf", "/local/run.sh"]
		# 	}

		# 	template {
		# 		destination = "local/run.sh"
		# 		perms = "544"
		# 		change_mode = "restart"
		# 		data = <<-EOF
		# 		echo 'Prestart'
		# 		sleep 5

		# 		echo "nats"
		# 		{{ range service "listen.nats" }}
		# 			echo "* {{ .Address }}:{{ .Port }}"
		# 		{{ end }}

		# 		echo 'Finished'
		# 		EOF
		# 	}

		# 	resources {
		# 		cpu = 10
		# 		memory = 50
		# 	}
		# }

		# task "poststop-test" {
		# 	lifecycle {
		# 		hook = "poststop"
		# 		sidecar = false
		# 	}

		# 	driver = "docker"

		# 	config {
		# 		image = "alpine:3.14"
		# 		entrypoint = ["/bin/sh", "-euf", "/local/run.sh"]
		# 	}

		# 	template {
		# 		destination = "local/run.sh"
		# 		perms = "544"
		# 		change_mode = "restart"
		# 		data = <<-EOF
		# 		echo 'Poststop'
		# 		sleep 5

		# 		echo "nats"
		# 		{{ range service "listen.nats" }}
		# 			echo "* {{ .Address }}:{{ .Port }}"
		# 		{{ end }}

		# 		echo 'Finished'
		# 		EOF
		# 	}

		# 	resources {
		# 		cpu = 10
		# 		memory = 50
		# 	}
		# }

		ephemeral_disk {
			size = 500
		}
	}
}
