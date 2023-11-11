locals {
	is_linode = var.region.provider == "linode"
}

resource "linode_sshkey" "server" {
	count = local.is_linode ? 1 : 0

	label = var.label
	ssh_key = chomp(data.tls_public_key.server.public_key_openssh)
}

resource "random_string" "server_root_pass" {
	count = local.is_linode ? 1 : 0

	length = 16
}

resource "linode_instance" "server" {
	count = local.is_linode ? 1 : 0

	lifecycle {
		ignore_changes = [authorized_keys]
	}

	label = var.label
	group = var.namespace
	region = var.region.provider_region
	type = var.size
	authorized_keys = [linode_sshkey.server[0].ssh_key]
	tags = var.tags
	private_ip = true
	backups_enabled = var.backup
}

resource "linode_instance_disk" "server_boot" {
	count = local.is_linode ? 1 : 0

	label = "boot"
	linode_id = linode_instance.server[0].id
	size = linode_instance.server[0].specs.0.disk - 512

	authorized_keys = [linode_sshkey.server[0].ssh_key]
	root_pass = random_string.server_root_pass[0].result
	image = "linode/debian11"
}

resource "linode_instance_disk" "server_swap" {
	count = local.is_linode ? 1 : 0

	label = "swap"
	linode_id = linode_instance.server[0].id
	size = 512
	filesystem = "swap"
}

resource "linode_instance_config" "server_boot_config" {
	count = local.is_linode ? 1 : 0

	lifecycle {
		ignore_changes = [
			booted,
		]
	}

	label = "boot_config"
	linode_id = linode_instance.server[0].id

	booted = true

	kernel = "linode/latest-64bit"

	root_device = "/dev/sda"

	devices {
		sda {
			disk_id = linode_instance_disk.server_boot[0].id
		}

		sdb {
			disk_id = linode_instance_disk.server_swap[0].id
		}
		
		# TODO: Make this sdc, sdd, sde, etc to support multiple volumes
		dynamic "sdc" {
			for_each = var.volumes

			content {
				volume_id = linode_volume.server[sdc.key].id
			}
		}
	}

	interface {
		purpose = "public"
	}

	interface {
		purpose = "vlan"
		label = "${var.namespace}-vlan"
		ipam_address = "${var.vlan.ip}/${var.region.vlan.prefix_len}"
	}
}

resource "linode_volume" "server" {
	for_each = {
		for k, v in var.volumes:
		k => v
		if local.is_linode
	}

	lifecycle {
		# TODO: SVC-2459
		# prevent_destroy = true
	}

	label = "${var.label}-${each.key}"
	region = var.region.provider_region
	size = each.value.size
}

resource "linode_firewall" "server" {
	count = local.is_linode ? 1 : 0

	label = var.label

	dynamic "inbound" {
		for_each = local.firewalls

		content {
			label = inbound.value.label
			action = "ACCEPT"
			protocol = upper(inbound.value.protocol)
			ports = inbound.value.ports

			ipv4 = inbound.value.inbound_ipv4_cidr
			ipv6 = inbound.value.inbound_ipv6_cidr
		}
	}

	inbound_policy = "DROP"

	outbound_policy = "ACCEPT"

	linodes = [linode_instance.server[0].id]
}

