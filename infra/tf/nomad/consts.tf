locals {
	# The region to deploy the main cluter to
	primary_dc = var.deploy_method_local ? "local" : var.primary_region

	# Shared config that gets passed to all jobspecs
	jobspec_shared = {
		node_class = var.deploy_method_local ? "local" : "svc"

		dns_config = <<EOF
dns {
	# HACK: Hardcode the interface that Docker binds to in order to be able to
	# resolve dnsmasq requests. The value should be:
	# servers = ["$${driver.docker.bridge_ip}"]
	servers = ["172.17.0.1"]
	searches = null
	options = null
}
EOF

		# Provide a Docker download token so we don't hit the download rate limit.
		#
		# https://docs.docker.com/docker-hub/download-rate-limit/#:~:text=For%20anonymous%20users%2C%20the%20rate,Enhanced%20Service%20Account%20add%2Don.
		docker_auth = var.authenticate_all_docker_hub_pulls ?  (
			<<-EOF
			auth {
				username = "${module.secrets.values["docker/registry/docker.io/username"]}"
				password = "${module.secrets.values["docker/registry/docker.io/password"]}"
			}
			EOF
		) : ""

		domain = {
			base = var.domain_main
			game = var.domain_cdn
			job = var.domain_job
		}
	}
}

