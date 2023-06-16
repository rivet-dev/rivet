# Uploads the grain config to the minion. This gets run any time the grain changes.

locals {
	grain = jsonencode({
		roles = var.roles

		rivet = {
			namespace = var.namespace

			region_id = var.server.region_id
			pool_id = var.server.pool_id
			version_id = var.server.version_id
			index = var.server.index

			name = var.server.name

			size = var.server.size

			provider_id = var.region.provider
			provider_region = var.region.provider_region
		}

		network = {
			vpc = var.vpc
		}

		nebula = var.nebula

		volumes = var.volumes
	})
}

resource "local_file" "upload_grain_local" {
	count = var.local_minion ? 1 : 0
	
	depends_on = [
		null_resource.install_salt_remote,
	]

	content = local.grain
	filename = "/etc/salt/grains"
}

resource "null_resource" "upload_grain_remote" {
	count = var.local_minion ? 0 : 1

	depends_on = [
		null_resource.install_salt_remote,
	]

	triggers = {
		server_id = var.minion_server_id
		grain = jsonencode(local.grain)
	}
	
	connection {
		type = "ssh"
		host = var.minion_host
		user = var.minion_user
		private_key = var.minion_private_key
	}

	provisioner "file" {
		content = local.grain
		destination = "/etc/salt/grains"
	}
}
