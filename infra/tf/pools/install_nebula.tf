module "install_nebula" {
	source = "../modules/install_nebula"

	for_each = var.servers

	install_local = false
	install_remote = true

	host = module.servers[each.key].host
	user = module.servers[each.key].user
	private_key_openssh = module.servers[each.key].private_key_openssh

	nebula_ca_cert = data.terraform_remote_state.nebula.outputs.nebula_ca_cert
	nebula_ca_key = data.terraform_remote_state.nebula.outputs.nebula_ca_key

	nebula_name = each.value.name
	nebula_ip = each.value.nebula_ip
	nebula_netmask = var.nebula_netmask
	nebula_groups = flatten([
		["pool:${each.value.pool_id}"],
		[
			for role in var.pools[each.value.pool_id].roles:
			"role:${role}"
		],
	])
	static_host_map = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.nebula_static_host_map : data.terraform_remote_state.master_local[0].outputs.nebula_static_host_map
	lighthouse_hosts = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.nebula_lighthouse_hosts : data.terraform_remote_state.master_local[0].outputs.nebula_lighthouse_hosts
	preferred_ranges = var.regions[each.value.region_id].preferred_subnets

	firewall = {
		conntrack = {
			tcp_timeout = "12m"
			udp_timeout = "3m"
			default_timeout = "10m"
		}

		outbound = [
			{ proto = "any", port = "any", host = "any" },
		]

		inbound = var.pools[each.value.pool_id].nebula_firewall_inbound
	}
}
