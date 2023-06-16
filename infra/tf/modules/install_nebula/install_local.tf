resource "local_file" "nebula_config" {
	count = var.install_local ? 1 : 0

	content = local.nebula_config_encoded
	filename = "/etc/nebula/config.yaml"
	directory_permission = "500"
}

resource "null_resource" "upload_nebula_config_local" {
	count = var.install_local ? 1 : 0

	depends_on = [
	  local_file.nebula_config,
	]

	triggers = {
		nebula_version = var.nebula_version
		config_encoded = md5(local.nebula_config_encoded)
	}

	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = templatefile("${path.module}/files/install_nebula.sh.tpl", {
			version = var.nebula_version
		})
	}
}
