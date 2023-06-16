module "install_salt_minion" {
	source = "../modules/install_salt_minion"

	for_each = var.servers

	depends_on = [
		module.install_nebula
	]

	namespace = var.namespace

	install_local = var.deploy_method_local
	install_remote = var.deploy_method_cluster

	master_host = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.salt_master_host : null
	master_user = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.salt_master_user : null
	master_private_key = var.deploy_method_cluster ? data.terraform_remote_state.master_cluster[0].outputs.salt_master_private_key_openssh : null
	master_nebula_ip = var.salt_master_nebula_ip

	minion_host = module.servers[each.key].host
	minion_user = module.servers[each.key].user
	minion_private_key = module.servers[each.key].private_key_openssh
	minion_server_id = module.servers[each.key].server_id

	roles = var.pools[each.value.pool_id].roles
	region = var.regions[each.value.region_id]
	server = each.value
	vpc = var.pools[each.value.pool_id].vpc ? {
		ips = [each.value.vpc_ip]
	} : null
	nebula = {
		ipv4 = each.value.nebula_ip
	}
	volumes = {
		for k, v in each.value.volumes:
		k => {
			size = v.size
			mount = true
		}
	}
}
