# Must be created in every namespace it is used in
resource "kubernetes_secret" "ingress_tls_cert" {
	for_each = toset(["traefik", "imagor", "rivet-service"])

	metadata {
		name = "ingress-tls-cloudflare-cert"
		namespace = each.value
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = cloudflare_origin_ca_certificate.rivet_gg.certificate
		"tls.key" = tls_private_key.cf_origin_rivet_gg.private_key_pem
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert" {
	for_each = toset(["traefik", "imagor", "rivet-service"])

	metadata {
		name = "ingress-tls-cloudflare-ca-cert"
		namespace = each.value
	}

	data = {
		"tls.ca" = local.cloudflare_ca_cert
	}
}

# resource "kubernetes_secret" "ingress_tls_cert_tunnel_server" {
# 	for_each = toset([
# 		for x in [kubernetes_namespace.traefik_tunnel]:
# 		x.metadata.0.name
# 	])

# 	metadata {
# 		name = "ingress-tls-cert-tunnel-server"
# 		namespace = each.value
# 	}

# 	type = "kubernetes.io/tls"

# 	data = {
# 		"tls.crt" = data.terraform_remote_state.tls.outputs.tls_cert_locally_signed_tunnel_server.cert_pem
# 		"tls.key" = data.terraform_remote_state.tls.outputs.tls_cert_locally_signed_tunnel_server.key_pem
# 	}
# } 
