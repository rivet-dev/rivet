module "servers" {
    source = "../modules/generic_server"

	for_each = var.servers

	namespace = var.namespace
	private_key_openssh = module.secrets.values["ssh/salt_minion/private_key_openssh"]

	region = var.regions[each.value.region_id]
	size = each.value.size
	label = each.value.name
	tags = each.value.tags
	vlan = {
		ip = each.value.vlan_ip
	}
	volumes = each.value.volumes
	firewall_inbound = var.pools[each.value.pool_id].firewall_inbound
}
