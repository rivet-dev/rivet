resource "tls_private_key" "locally_signed_tunnel_server" {
	algorithm = "RSA"
}

resource "tls_cert_request" "locally_signed_tunnel_server" {
	key_algorithm   = tls_private_key.locally_signed_tunnel_server.algorithm
	private_key_pem = tls_private_key.locally_signed_tunnel_server.private_key_pem

	subject {
		common_name  = "Tunnel Server"
		organization = "Rivet Gaming, Inc."
	}

	# TODO:
	dns_names = ["tunnel.rivet.gg"]
}

resource "tls_locally_signed_cert" "locally_signed_tunnel_server" {
	cert_request_pem = tls_cert_request.locally_signed_tunnel_server.cert_request_pem
	ca_key_algorithm   = "RSA"
	ca_private_key_pem = tls_private_key.root_ca.private_key_pem
	ca_cert_pem        = tls_self_signed_cert.root_ca.cert_pem
  
	validity_period_hours = 8760 # 1 year

	allowed_uses = [
		"server_auth"
	]
}

resource "kubernetes_secret" "ingress_tls_cert_tunnel_server" {
	type = "kubernetes.io/tls"
	for_each = toset(["traefik-tunnel", "nomad", "rivet-service"])

	metadata {
		name = "ingress-tls-cert-tunnel-server"
		namespace = each.value
		labels = {
			"traefik-instance" = "tunnel"
		}
	}

	data = {
		"tls.crt" = tls_locally_signed_cert.locally_signed_tunnel_server.cert_pem
		"tls.key" = tls_private_key.locally_signed_tunnel_server.private_key_pem
	}
}
