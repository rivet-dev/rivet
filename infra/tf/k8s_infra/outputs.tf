output "traefik_external_ip" {
	value = (
		var.deploy_method_cluster ?
			data.kubernetes_service.traefik.status[0].load_balancer[0].ingress[0].hostname :
			var.public_ip
	)
}

output "traefik_tunnel_external_ip" {
	value = (
		var.deploy_method_cluster && var.edge_enabled ?
			data.kubernetes_service.traefik_tunnel.0.status[0].load_balancer[0].ingress[0].hostname :
			var.public_ip
	)
}
