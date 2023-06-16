# TODO: How do upgrade SaltStack?
resource "null_resource" "install_salt_local" {
	count = var.install_local ? 1 : 0

	triggers = {
		version = local.salt_version
	}

	connection {
		type = "ssh"
		host = var.host
		user = var.user
		private_key = var.private_key
	}
	
	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = templatefile("${path.module}/files/install_salt.sh.tpl", {
			salt_master_name = var.salt_master_name
            version = local.salt_version
		})
	}
}

resource "local_file" "salt_config_local" {
	count = var.install_local ? 1 : 0

	content = local.master_conf
	filename = "/etc/salt/master.d/master.conf"
}

resource "null_resource" "salt_master_config_local" {
	count = var.install_local ? 1 : 0

	depends_on = [
		null_resource.install_salt_local,
	]

	# Setup directories for uploads
	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = "mkdir -p /srv/terraform"
	}
}

# Restart if the core configs change
resource "null_resource" "salt_master_restart_local" {
	count = var.install_local ? 1 : 0

	depends_on = [
		local_file.salt_config_local,
		null_resource.salt_master_config_local
	]

	triggers = {
		master_conf = local.master_conf
	}

	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = "systemctl restart salt-master"
	}
}

