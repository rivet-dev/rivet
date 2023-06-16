job "imagor:${dc}" {
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

	priority = 35

	group "imagor" {
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
				to = 8000
			}
		}

		service {
			name = "imagor"
			tags = [
				"http",

				%{ for prefix in ["traefik-ing-px", "traefik-local"] }
				"${prefix}.enable=true",


				# middlewares.imagor-cors
				"${prefix}.http.middlewares.imagor-cors.headers.accessControlAllowMethods=GET, OPTIONS",
				"${prefix}.http.middlewares.imagor-cors.headers.accessControlAllowOriginList=https://${shared.domain.base}",
				"${prefix}.http.middlewares.imagor-cors.headers.accessControlMaxAge=300",

				# middlewares.imagor-cors-game
				"${prefix}.http.middlewares.imagor-cors-game.headers.accessControlAllowMethods=GET, OPTIONS",
				"${prefix}.http.middlewares.imagor-cors-game.headers.accessControlAllowOriginList=*",
				"${prefix}.http.middlewares.imagor-cors-game.headers.accessControlMaxAge=300",

				# middlewares.imagor-cdn
				"${prefix}.http.middlewares.imagor-cdn.chain.middlewares=imagor-cdn-retry, imagor-cdn-cache-control",

				# middlewares.imagor-cdn-retry
				"${prefix}.http.middlewares.imagor-cdn-retry.retry.attempts=4",
				"${prefix}.http.middlewares.imagor-cdn-retry.retry.initialInterval=1s",

				# middlewares.imagor-cdn-cache-control
				#
				# `Cache-Control`:
				# See
				# https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#caching_static_assets
				# and
				# https://imagekit.io/blog/ultimate-guide-to-http-caching-for-static-assets/
				#
				# `Vary`:
				# We serve different responses if the client accepts WebP/AVIF
				# Disabled for now. See RIV-556.
				"${prefix}.http.middlewares.imagor-cdn-cache-control.headers.customResponseHeaders.Cache-Control=public, max-age=604800, immutable",
				# "${prefix}.http.middlewares.imagor-cdn-cache-control.headers.customResponseHeaders.Vary=Accept",

				%{ for preset in imagor_presets }
				# routers.imagor-${preset.key}
				"${prefix}.http.routers.imagor-${preset.key}.entrypoints=lb-443",
				"${prefix}.http.routers.imagor-${preset.key}.priority=${preset.priority}",
				%{ if preset.query != null }
				"${prefix}.http.routers.imagor-${preset.key}.rule=(Host(`media.${shared.domain.base}`) || HostRegexp(`media.{region:.+}.${shared.domain.base}`)) && Path(`${preset.path}`) && Query(%{ for x in preset.query }`${x[0]}=${x[1]}`,%{ endfor })",
				%{ else }
				"${prefix}.http.routers.imagor-${preset.key}.rule=(Host(`media.${shared.domain.base}`) || HostRegexp(`media.{region:.+}.${shared.domain.base}`)) && Path(`${preset.path}`)",
				%{ endif }
				%{ if preset.game_cors }
				"${prefix}.http.routers.imagor-${preset.key}.middlewares=imagor-${preset.key}-path, imagor-cors-game, imagor-cdn",
				%{ else }
				"${prefix}.http.routers.imagor-${preset.key}.middlewares=imagor-${preset.key}-path, imagor-cors, imagor-cdn",
				%{ endif }
				"${prefix}.http.routers.imagor-${preset.key}.tls=true",

				# middlewares.imagor-${preset.key}-path
				"${prefix}.http.middlewares.imagor-${preset.key}-path.replacePathRegex.regex=${preset.path_regexp}",
				"${prefix}.http.middlewares.imagor-${preset.key}-path.replacePathRegex.replacement=${replace(preset.path_regex_replacement, "$${", "$$${")}",
				%{ endfor }

				%{ endfor }

			]
			port = "http"
			address_mode = "host"

			check {
				name = "healthy"
				type = "http"
				port = "http"
				path = "/"
				interval = "5s"
				timeout = "2s"
				on_update = "ignore"
			}

			check {
				name = "e2e healthy"
				type = "http"
				port = "http"
				path = "/unsafe/16x16/filters:strip_exif():strip_icc():format(jpeg):quality(80):background_color(2a2a2a)/https%3A%2F%2Frivet-assets.s3.us-west-004.backblazeb2.com%2Fhealthcheck.png"
				interval = "15s"
				timeout = "10s"
				on_update = "require_healthy"

				check_restart {
					limit = 2
				}
			}
		}

		task "node" {
			driver = "docker"

			config {
				image = "shumc/imagor:1.4.4"
				${shared.docker_auth}

				ports = ["http"]
			}

			env {
				PORT = "$${NOMAD_PORT_http}"

				# Unsafe is fine since we don't expose Imagor publicly and use explicit
				# rules Traefik to filter what requests can be made.
				IMAGOR_UNSAFE = "1"

				# Support more formats
				# Disabled for now. See RIV-556.
				# IMAGOR_AUTO_AVIF = "1"
				# IMAGOR_AUTO_WEBP = "1"

				# We don't need a storage config since ATS is responsible for saving files from S3 within the cluster

				# TODO: This might be incredibly slow since we have to write & fetch each individual size all the way from Backblaze without using an ATS cache

				# Configure fetching sized images from Backblaze
				S3_RESULT_STORAGE_ENDPOINT = "${result_storage_s3_endpoint}"
				AWS_RESULT_STORAGE_REGION = "${result_storage_s3_region}"
				AWS_RESULT_STORAGE_ACCESS_KEY_ID = "${result_storage_s3_access_key_id}"
				AWS_RESULT_STORAGE_SECRET_ACCESS_KEY = "${result_storage_s3_secret_access_key}"
				S3_RESULT_STORAGE_BUCKET = "${result_storage_s3_bucket}"
				S3_RESULT_STORAGE_EXPIRATION = "48h"
				S3_FORCE_PATH_STYLE = "1"
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

		ephemeral_disk {
			size = ${ephemeral_disk}
		}
	}
}
