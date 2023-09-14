resource "null_resource" "install" {
	for_each = var.servers

	triggers = {
		install_script = md5(each.value.install_script)
	}

	connection {
		type = "ssh"
		host = module.servers[each.key].host
		user = module.servers[each.key].user
		private_key = module.servers[each.key].private_key_openssh
	}

	provisioner "remote-exec" {
		inline = [
			each.value.install_script
		]
	}
}
