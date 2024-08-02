# MARK: Private key
resource "tls_private_key" "acme_account_key" {
	# Must be EC key to work with acme_lib
	algorithm = "ECDSA"
	ecdsa_curve = "P256"
}

# MARK: Registration
resource "acme_registration" "main" {
	account_key_pem = tls_private_key.acme_account_key.private_key_pem
	email_address = "letsencrypt@rivet.gg"
}

