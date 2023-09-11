output "traefik_external_ip" {
	value = data.kubernetes_service.traefik.status[0].load_balancer[0].ingress[0].hostname
}

