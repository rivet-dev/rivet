output "tunnel_name" {
	value = var.name
}

output "tunnel_id" {
	value = cloudflare_tunnel.tunnel.id
}

output "cert_json" {
	value = local.cert_json
}

output "ingress_json" {
	value = jsonencode(flatten([
		[
			for k, v in var.ingress:
			{
				hostname = k
				service = v.service
			}
		],
		[
			{
				service = "http_status:404"
			}
		],
	]))
}

