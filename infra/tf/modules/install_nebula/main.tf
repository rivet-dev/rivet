terraform {
	required_providers {
		nebula = {
			source = "TelkomIndonesia/nebula"
			version = "0.3.1"
		}
	}
}

locals {
	nebula_config_encoded = jsonencode({
		pki = {
			ca = var.nebula_ca_cert
			cert = nebula_certificate.node.cert
			key = nebula_certificate.node.key
		}
		static_host_map = var.static_host_map
		lighthouse = {
			am_lighthouse = var.is_lighthouse
			hosts = var.is_lighthouse ? null : sort(var.lighthouse_hosts)
		}
		listen = {
			host = "0.0.0.0"
			port = 4242
		}
		punchy = {
			punch = true
		}
		
		# Recommend using LAN if on the same network
		preferred_ranges = var.preferred_ranges

		tun = {
			dev = "nebula0"
		}

		logging = {
			level = "info"
			format = "json"
		}

		stats = {
			type = "prometheus"
			listen = "${var.nebula_ip}:4280"
			path = "/metrics"
			namespace = "nebula"
			interval = "60s"
		}

		firewall = var.firewall
	})
}

resource "nebula_certificate" "node" {
	name = var.nebula_name
	ip = "${var.nebula_ip}/${var.nebula_netmask}"
	groups = var.nebula_groups

	ca_cert = var.nebula_ca_cert
	ca_key = var.nebula_ca_key
}
