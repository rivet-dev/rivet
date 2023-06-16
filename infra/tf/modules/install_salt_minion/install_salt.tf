locals {
	# Config passed to the installer
	minion_config = {
		# The grains will be updated in the next step.
		grains = {}

		# Prevents frequent disconnection of minions, see
		# https://github.com/saltstack/salt/issues/38157#issuecomment-396577378
		tcp_keepalive_idle = 60
	}

	salt_version = "3006.1"
}

# This gets run only once.
resource "null_resource" "install_salt_remote" {
	count = var.skip_install ? 0 : 1

	triggers = {
		server_id = var.minion_server_id
		version = local.salt_version
	}

	connection {
		type = "ssh"
		host = var.minion_host
		user = var.minion_user
		private_key = var.minion_private_key
		# Servers can sometimes take a long time to boot
		timeout = "10m"
	}

	provisioner "remote-exec" {
		inline = [templatefile("${path.module}/files/install_salt.sh.tpl", {
			name = var.server.name
			master_ip_address = var.master_nebula_ip
			minion_config = jsonencode(local.minion_config)
			version = local.salt_version
		})]
	}
}
