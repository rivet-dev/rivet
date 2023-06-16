locals {
	nebula_lighthouse_name = "${var.namespace}-nebula-lighthouse"

	nebula_static_host_map = {
		"${var.nebula_lighthouse_nebula_ip}" = ["${module.nebula_lighthouse_server.host}:4242"]
	}

	nebula_lighthouse_hosts = [var.nebula_lighthouse_nebula_ip]
}

module "nebula_lighthouse_server" {
    source = "../modules/generic_server"

	namespace = var.namespace
	private_key_openssh = module.secrets.values["ssh/nebula_lighthouse/private_key_openssh"]

	region = var.regions[var.primary_region]
	size = var.nebula_lighthouse_size
	label = local.nebula_lighthouse_name
	tags = [
		var.namespace,
		"${var.namespace}-${var.primary_region}",
		"${var.namespace}-nebula-lighthouse",
		"${var.namespace}-${var.primary_region}-nebula-lighthouse",
	]
	backup = true

	firewall_inbound = [
		{
			label = "nebula-udp"
			ports = "4242"
			protocol = "udp"
			inbound_ipv4_cidr = ["0.0.0.0/0"]
			inbound_ipv6_cidr = ["::/0"]
		},
		{
			label = "nebula-tcp"
			ports = "4242"
			protocol = "tcp"
			inbound_ipv4_cidr = ["0.0.0.0/0"]
			inbound_ipv6_cidr = ["::/0"]
		},
	]
}

module "install_nebula_lighthouse" {
	source = "../modules/install_nebula"

	depends_on = [
		module.nebula_lighthouse_server,
	]

	install_local = false
	install_remote = true

	host = module.nebula_lighthouse_server.host
	user = module.nebula_lighthouse_server.user
	private_key_openssh = module.nebula_lighthouse_server.private_key_openssh

	nebula_ca_cert = data.terraform_remote_state.nebula.outputs.nebula_ca_cert
	nebula_ca_key = data.terraform_remote_state.nebula.outputs.nebula_ca_key

	nebula_name = "${var.namespace}-nebula-lighthouse"
	nebula_ip = var.nebula_lighthouse_nebula_ip
	nebula_netmask = var.nebula_netmask
	nebula_groups = [
		"pool:neb-lh",
		"role:nebula-lighthouse",
	]
	is_lighthouse = true
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

		inbound = [
			# Monitoring
			{ proto = "tcp", port = "4280", group = "role:prometheus" }
		]
	}
}
