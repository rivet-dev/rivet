# MARK: Servers
output "servers" {
	value = {
		for server_id, server in var.servers:
		server_id => merge(server, {
			public_ipv4 = module.servers[server_id].public_ipv4
		})
	}
}
