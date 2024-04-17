locals {
	tls_cert_locally_signed_tunnel_server = {
		cert_pem = tls_locally_signed_cert.locally_signed_tunnel_server.cert_pem
		key_pem = tls_private_key.locally_signed_tunnel_server.private_key_pem
	}

	tls_cert_locally_signed_job = {
		cert_pem = tls_locally_signed_cert.locally_signed_client["job"].cert_pem
		key_pem = tls_private_key.locally_signed_client["job"].private_key_pem
	}

	tls_cert_locally_signed_gg = {
		cert_pem = tls_locally_signed_cert.locally_signed_client["gg"].cert_pem
		key_pem = tls_private_key.locally_signed_client["gg"].private_key_pem
	}
}

# MARK: Write secrets
output "tls_cert_locally_signed_tunnel_server" {
	value = local.tls_cert_locally_signed_tunnel_server
	sensitive = true
}

output "tls_cert_locally_signed_job" {
	value = local.tls_cert_locally_signed_job
	sensitive = true
}

output "tls_cert_locally_signed_gg" {
	value = local.tls_cert_locally_signed_gg
	sensitive = true
}

output "root_ca_cert_pem" {
	value = tls_self_signed_cert.root_ca.cert_pem
}

output "acme_account_private_key_pem" {
	value = tls_private_key.acme_account_key.private_key_pem
	sensitive = true
}
