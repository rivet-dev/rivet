resource "null_resource" "server_gc" {
	depends_on = [
		module.servers,
	]

	triggers = {
		servers = var.servers
	}

	master_host = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.salt_master_host : null
	
	provisioner "local-exec" {
		interpreter = ["/usr/bin/env", "-i", "bash", "-c"]
		command = "salt-key -L | grep -v ':' | grep -vx \"${join(keys(var.servers), "\\|")}\" | xargs -L 1 salt-key -d"
	}
}
