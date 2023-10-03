locals {
	tls_cert_cloudflare_rivet_gg = {
		cert_pem = cloudflare_origin_ca_certificate.rivet_gg.certificate
		key_pem = tls_private_key.cf_origin_rivet_gg.private_key_pem
	}

	tls_cert_letsencrypt_rivet_gg ={
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
		cert_pem = tls_locally_signed_cert.locally_signed_tunnel_server.ca_cert_pem
		key_pem = tls_locally_signed_cert.locally_signed_tunnel_server.ca_private_key_pem
	}

	tls_cert_locally_signed_nomad_client = {
		cert_pem = tls_locally_signed_cert.locally_signed_nomad_client.ca_cert_pem
		key_pem = tls_locally_signed_cert.locally_signed_nomad_client.ca_private_key_pem
	}

	tls_cert_locally_signed_game_guard = {
		cert_pem = tls_locally_signed_cert.locally_signed_game_guard.ca_cert_pem
		key_pem = tls_locally_signed_cert.locally_signed_game_guard.ca_private_key_pem
	}
}

output "tls_cert_cloudflare_rivet_gg" {
	value = local.tls_cert_cloudflare_rivet_gg
	sensitive = true
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

output "tls_cert_cloudflare_ca" {
	value = local.cloudflare_ca_cert
	sensitive = true
}

output "tls_cert_locally_signed_tunnel_server" {
	value = local.tls_cert_locally_signed_tunnel_server
	sensitive = true
}

output "tls_cert_locally_signed_nomad_client" {
	value = local.tls_cert_locally_signed_nomad_client
	sensitive = true
}