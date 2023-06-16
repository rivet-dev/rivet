resource "null_resource" "upload_nebula_config_remote" {
	count = var.install_remote ? 1 : 0

	triggers = {
		nebula_version = var.nebula_version
		config_encoded = md5(local.nebula_config_encoded)
	}

	connection {
		type = "ssh"
		host = var.host
		user = var.user
		private_key = var.private_key_openssh
	}
	
	provisioner "remote-exec" {
		inline = [
			<<-EOF
			# Create configuration directory for Nebula if does not exist
			mkdir -p /etc/nebula
			chmod 500 /etc/nebula
			EOF
		]
	}

	provisioner "file" {
		destination = "/etc/nebula/config.yaml"
		content = local.nebula_config_encoded
	}

	provisioner "remote-exec" {
		inline = [
			templatefile("${path.module}/files/install_nebula.sh.tpl", {
				version = var.nebula_version
			})
		]
	}
}
