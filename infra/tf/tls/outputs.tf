locals {
	tls_cert_letsencrypt_rivet_gg = {
		# Build full chain by concatenating the certificate with issuer.
		#
		# See
		# https://registry.terraform.io/providers/vancluever/acme/latest/docs/resources/certificate#certificate_pem
		cert_pem = "${acme_certificate.rivet_gg.certificate_pem}${acme_certificate.rivet_gg.issuer_pem}"
		key_pem = acme_certificate.rivet_gg.private_key_pem
	}

	tls_cert_letsencrypt_rivet_game = {
		# See above
		cert_pem = "${acme_certificate.rivet_game.certificate_pem}${acme_certificate.rivet_game.issuer_pem}"
		key_pem = acme_certificate.rivet_game.private_key_pem
	}

	tls_cert_letsencrypt_rivet_job = {
		# See above
		cert_pem = "${acme_certificate.rivet_job.certificate_pem}${acme_certificate.rivet_job.issuer_pem}"
		key_pem = acme_certificate.rivet_job.private_key_pem
	}

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
output "tls_cert_letsencrypt_rivet_gg" {
	value = local.tls_cert_letsencrypt_rivet_gg
	sensitive = true
}

output "tls_cert_letsencrypt_rivet_game" {
	value = local.tls_cert_letsencrypt_rivet_game
	sensitive = true
}

output "tls_cert_letsencrypt_rivet_job" {
	value = local.tls_cert_letsencrypt_rivet_job
	sensitive = true
}

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

