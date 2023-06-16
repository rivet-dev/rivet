terraform {
	required_providers {
		linode = {
			source = "linode/linode"
			version = "1.29.2"
		}
	}
}

locals {
	firewalls = flatten([
		// Configured ports
		var.firewall_inbound,

		// Default ports
		[
			{
				label = "ssh"
				ports = "22"
				protocol = "tcp"

				# TODO: Configure default inbound for SSH
				inbound_ipv4_cidr = ["0.0.0.0/0"]
				inbound_ipv6_cidr = ["::/0"]
			}
		],
	])
}
