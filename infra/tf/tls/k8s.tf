# Must be created in every namespace it is used in
resource "kubernetes_secret" "ingress_tls_cert" {
	for_each = toset(["traefik", "imagor"])

	metadata {
		name = "ingress-tls-cert"
		namespace = each.value
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = cloudflare_origin_ca_certificate.rivet_gg.certificate
		"tls.key" = tls_private_key.cf_origin_rivet_gg.private_key_pem
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert" {
	metadata {
		name = "ingress-tls-ca-cert"
		namespace = "traefik"
	}

	data = {
		"tls.ca" = local.cloudflare_ca_cert
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert_locally_signed" {
	metadata {
		name = "ingress-tls-ca-cert-locally-signed"
		namespace = "traefik-tunnel"
	}

	data = {
		"tls.ca" = tls_self_signed_cert.root_ca.cert_pem
	}
}

resource "kubernetes_secret" "ingress_tls_cert_tunnel_server" {
	metadata {
		name = "ingress-tls-cert-tunnel-server"
		namespace = "traefik-tunnel"
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = tls_locally_signed_cert.locally_signed_tunnel_server.cert_pem
		"tls.key" = tls_private_key.locally_signed_tunnel_server.private_key_pem
	}
} 
