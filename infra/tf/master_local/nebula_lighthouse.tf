locals {
	nebula_static_host_map = {
		"${var.nebula_lighthouse_nebula_ip}" = ["${var.public_ip}:4242"]
	}

	nebula_lighthouse_hosts = [var.nebula_lighthouse_nebula_ip]
}

# Installs as both a lighthouse and client locally
module "install_nebula" {
	source = "../modules/install_nebula"

	install_local = true
	install_remote = false

	nebula_ca_cert = data.terraform_remote_state.nebula.outputs.nebula_ca_cert
	nebula_ca_key = data.terraform_remote_state.nebula.outputs.nebula_ca_key

	nebula_name = "${var.namespace}-local"
	nebula_ip = var.nebula_lighthouse_nebula_ip
	nebula_netmask = var.nebula_netmask
	nebula_groups = [
		"pool:neb-lh",
		"pool:salt-master",
		"role:nebula-lighthouse",
		"role:salt-master",
	]
	is_lighthouse = true
	static_host_map = local.nebula_static_host_map
	lighthouse_hosts = local.nebula_lighthouse_hosts
	preferred_ranges = var.local_preferred_subnets
	firewall = {
		conntrack = {
			tcp_timeout = "12m"
			udp_timeout = "3m"
			default_timeout = "10m"
		}

		outbound = [
			{ proto = "any", port = "any", host = "any" },
		]

		inbound = flatten([
			var.pools["local"].nebula_firewall_inbound,

			// See also tf/master_cluster/salt_master.tf
			[
				// ICMP to allow `ping` to work
				{ proto = "icmp", port = "any", host = "any" },
				// Nebula Prometheus
				{ proto = "tcp", port = "4280", group = "role:prometheus" },
				// Node exporter
				{ proto = "tcp", port = "9100", group = "role:prometheus" },
				// SaltStack
				{ proto = "tcp", port = "4505-4506", host = "any" },
			]
		])
	}
}
