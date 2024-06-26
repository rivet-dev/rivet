output "traefik_external_ip" {
	value = var.public_ip
}

output "repo_host" {
	value = local.repo_host
}

output "repo_port" {
	value = local.repo_port
}
