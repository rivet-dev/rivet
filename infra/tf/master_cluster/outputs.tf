# MARK: Salt Master
output "salt_master_host" {
	value = module.salt_master_server.host
}

output "salt_master_user" {
	value = module.salt_master_server.user
}

output "salt_master_private_key_openssh" {
	value = module.salt_master_server.private_key_openssh
	sensitive = true
}

# MARK: Nebula Lighthouse
output "nebula_lighthouse_host" {
	value = module.nebula_lighthouse_server.host
}

output "nebula_lighthouse_user" {
	value = module.nebula_lighthouse_server.user
}

output "nebula_lighthouse_private_key_openssh" {
	value = module.nebula_lighthouse_server.private_key_openssh
	sensitive = true
}

# MARK: Nebula Lighthoug
output "nebula_static_host_map" {
    value = local.nebula_static_host_map
}

output "nebula_lighthouse_hosts" {
    value = local.nebula_lighthouse_hosts
}
