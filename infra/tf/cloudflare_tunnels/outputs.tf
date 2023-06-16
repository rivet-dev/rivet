output "salt_output" {
	value = {
		tunnels = {
			for k, v in module.cloudflare_tunnels:
			k => {
				tunnel_name = v.tunnel_name
				tunnel_id = v.tunnel_id
				cert_json = v.cert_json
				ingress_json = v.ingress_json
			}
		}
	}
    sensitive = true
}

