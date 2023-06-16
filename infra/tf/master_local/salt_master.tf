locals {
	salt_master_name = "${var.namespace}-local"
}

module "install_salt_master" {
	source = "../modules/install_salt_master"

	depends_on = [
	  module.install_nebula,
	]

    install_local = true
	install_remote = false

	salt_master_name = local.salt_master_name
}

# Configure the local machine as a Salt minion so we can install the required
# software on it.
module "install_salt_minion" {
	source = "../modules/install_salt_minion"

	depends_on = [module.install_salt_master]

	namespace = var.namespace

	install_local = true
	install_remote = false

	master_nebula_ip = var.salt_master_nebula_ip

	skip_install = true
	local_minion = true
	minion_server_id = "local"

	roles = var.pools["local"].roles
	region = {
		provider = "local"
		provider_region = "local"
	}
	server = {
		region_id = "local"
		pool_id = "local"
		version_id = "01"
		index = 0
		name = local.salt_master_name
		size = "local"

	}
	# TODO:
	# vpc = var.pools["local"].vpc ? {
	# 	ips = ["TODO"]
	# } : null
	vpc = null
	nebula = {
		ipv4 = var.salt_master_nebula_ip
	}
	volumes = {
		for volume_id, _ in var.pools["local"].volumes:
		volume_id => {
			# TODO: Configure default size for each volume
			size = 8
			mount = false
		}
	}
}

