resource "null_resource" "accept_local" {
	count = var.install_local ? 1 : 0

	depends_on = [
		local_file.upload_grain_local,
		null_resource.upload_grain_remote,
	]

	triggers = {
		server_id = var.minion_server_id
		grain = local.grain
	}
	
	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = templatefile("${path.module}/files/accept_and_refresh_minion.sh.tpl", {
			name = var.server.name
		})
	}
}

resource "null_resource" "accept_remote" {
	count = var.install_remote ? 1 : 0

	depends_on = [
		local_file.upload_grain_local,
		null_resource.upload_grain_remote,
	]

	triggers = {
		server_id = var.minion_server_id
		grain = local.grain
	}
	
	provisioner "remote-exec" {
		connection {
			type = "ssh"
			host = var.master_host
			user = var.master_user
			private_key = var.master_private_key
		}

		inline = [
			templatefile("${path.module}/files/accept_and_refresh_minion.sh.tpl", {
				name = var.server.name
			})
		]
	}
}
