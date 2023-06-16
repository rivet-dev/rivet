output "server_id" {
	value = coalesce(
        var.region.provider == "linode" ? "${var.region.provider}:${linode_instance.server[0].id}" : null,
    )
}

output "host" {
	value = coalesce(
        var.region.provider == "linode" ? linode_instance.server[0].ip_address : null,
    )
}

output "user" {
	value = "root"
}

output "private_key_openssh" {
	value = var.private_key_openssh
	sensitive = true
}


output "public_ipv4" {
	value = coalesce(
        var.region.provider == "linode" ? linode_instance.server[0].ip_address : null,
    )
}
