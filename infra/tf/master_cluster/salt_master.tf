locals {
	salt_master_name = "${var.namespace}-salt-master"
}

module "salt_master_server" {
    source = "../modules/generic_server"

	namespace = var.namespace
	private_key_openssh = module.secrets.values["ssh/salt_master/private_key_openssh"]

	region = var.regions[var.primary_region]
	size = var.salt_master_size
	label = local.salt_master_name
	tags = [
		var.namespace,
		"${var.namespace}-${var.primary_region}",
		"${var.namespace}-salt-master",
		"${var.namespace}-${var.primary_region}-salt-master",
	]
	backup = true
}

module "install_nebula_salt_master" {
	source = "../modules/install_nebula"

	depends_on = [
		module.install_nebula_lighthouse,
		module.salt_master_server,
	]

	install_local = false
	install_remote = true

	host = module.salt_master_server.host
	user = module.salt_master_server.user
	private_key_openssh = module.salt_master_server.private_key_openssh

	nebula_ca_cert = data.terraform_remote_state.nebula.outputs.nebula_ca_cert
	nebula_ca_key = data.terraform_remote_state.nebula.outputs.nebula_ca_key

	nebula_name = "${var.namespace}-salt-master"
	nebula_ip = var.salt_master_nebula_ip
	nebula_netmask = var.nebula_netmask
	nebula_groups = [
		"pool:salt-master",
		"role:salt-master",
	]
	static_host_map = local.nebula_static_host_map
	lighthouse_hosts = local.nebula_lighthouse_hosts
	preferred_ranges = var.regions[var.primary_region].preferred_subnets
	firewall = {
		conntrack = {
			tcp_timeout = "12m"
			udp_timeout = "3m"
			default_timeout = "10m"
		}

		outbound = [
			{ proto = "any", port = "any", host = "any" },
		]

		// See also tf/master_local/salt_master.tf
		inbound = [
			// ICMP to allow `ping` to work
			{ proto = "any", port = "any", host = "any" },
			// Nebula Prometheus
			{ proto = "tcp", port = "4280", group = "role:prometheus" },
			// Node exporter
			{ proto = "tcp", port = "9100", group = "role:prometheus" },
			// SaltStack
			{ proto = "tcp", port = "4505-4506", host = "any" },
		]
	}
}

module "install_salt_master" {
	source = "../modules/install_salt_master"

	depends_on = [module.install_nebula_salt_master]

	host = module.salt_master_server.host
	user = module.salt_master_server.user
	private_key = module.salt_master_server.private_key_openssh

    install_local = false
	install_remote = true

	salt_master_name = local.salt_master_name
}
