locals {
	dev_tunnel_name = "${var.namespace}-dev-tunnel"
}

resource "random_string" "tunnel_suffix" {
	length = 8
	special = false
	upper = false
	lower = true
	numeric  = true
}

resource "random_password" "password" {
	length  = 16
	special = true
	override_special = "_%@"
}

resource "linode_instance" "tunnel" {
	image = "linode/debian11"
	label = local.dev_tunnel_name
	region = "us-west"
	type = "g6-nanode-1"
	authorized_keys = [trimspace(tls_private_key.ssh_key.public_key_openssh)]
	root_pass = random_password.password.result
	tags = ["rivet-${var.namespace}", "${var.namespace}-dev-tunnel"]
}

resource "linode_firewall" "tunnel_firewall" {
	label = local.dev_tunnel_name

	inbound_policy = "DROP"
	outbound_policy = "ACCEPT"

	inbound {
		label = "ssh"
		action = "ACCEPT"
		protocol = "TCP"
		ports = "22"
		ipv4 = ["0.0.0.0/0"]
		ipv6 = ["::/0"]
	}

	inbound {
		label = "http"
		action = "ACCEPT"
		protocol = "TCP"
		ports = var.api_http_port
		ipv4 = ["0.0.0.0/0"]
		ipv6 = ["::/0"]
	}

	dynamic "inbound" {
		for_each = var.api_https_port != null ? [1] : []
		content {
			label = "https"
			action = "ACCEPT"
			protocol = "TCP"
			ports = var.api_https_port
			ipv4 = ["0.0.0.0/0"]
			ipv6 = ["::/0"]
		}
	}

	inbound {
		label = "tunnel"
		action = "ACCEPT"
		protocol = "TCP"
		ports = var.tunnel_port
		ipv4 = ["0.0.0.0/0"]
		ipv6 = ["::/0"]
	}

	dynamic "inbound" {
		for_each = var.minio_port != null ? [1] : []
		content {
			label = "minio"
			action = "ACCEPT"
			protocol = "TCP"
			ports = var.minio_port
			ipv4 = ["0.0.0.0/0"]
			ipv6 = ["::/0"]
		}
	}

	linodes = [linode_instance.tunnel.id]
}

