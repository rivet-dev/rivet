resource "kubernetes_namespace" "infra" {
	for_each = toset(["traefik", "imagor"])

	metadata {
		name = each.key
	}
}

# Must be created in every namespace it is used in
resource "kubernetes_secret" "ingress_tls_cert" {
	for_each = toset([
		for x in [kubernetes_namespace.infra]:
		x.metadata.0.name
	])

	metadata {
		name = "ingress-tls-cert"
		namespace = each.value
	}

	type = "kubernetes.io/tls"

	data = {
		"tls.crt" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_rivet_gg.cert_pem
		"tls.key" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_rivet_gg.key_pem
	}
}

resource "kubernetes_secret" "ingress_tls_ca_cert" {
	metadata {
		name = "ingress-tls-ca-cert"
		namespace = kubernetes_namespace.infra["traefik"].metadata.0.name
	}

	data = {
		"tls.ca" = data.terraform_remote_state.tls.outputs.tls_cert_cloudflare_ca
	}
}
