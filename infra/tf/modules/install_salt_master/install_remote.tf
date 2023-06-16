# TODO: How do upgrade SaltStack?
resource "null_resource" "install_salt_remote" {
	count = var.install_remote ? 1 : 0

	triggers = {
		version = local.salt_version
	}

	connection {
		type = "ssh"
		host = var.host
		user = var.user
		private_key = var.private_key
	}
	
	provisioner "remote-exec" {
		inline = [
			templatefile("${path.module}/files/install_salt.sh.tpl", {
				salt_master_name = var.salt_master_name
				version = local.salt_version
			})
		]
	}
}

resource "null_resource" "salt_master_config_remote" {
	count = var.install_remote ? 1 : 0

	depends_on = [
		null_resource.install_salt_remote,
	]

	triggers = {
		host = var.host
		master_conf = local.master_conf
	}

	connection {
		type = "ssh"
		host = var.host
		user = var.user
		private_key = var.private_key
	}
	
	# Setup directories for uploads
	provisioner "remote-exec" {
		inline = ["mkdir -p /srv/terraform"]
	}

	provisioner "file" {
		content = local.master_conf
		destination = "/etc/salt/master.d/master.conf"
	}
}

# Restart if the core configs change
resource "null_resource" "salt_master_restart_remote" {
	count = var.install_remote ? 1 : 0

	depends_on = [null_resource.salt_master_config_remote]

	triggers = {
		host = var.host
		master_conf = local.master_conf
	}

	connection {
		type = "ssh"
		host = var.host
		user = "root"
		private_key = var.private_key
	}

	provisioner "remote-exec" {
		inline = ["systemctl restart salt-master"]
	}
}

