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
