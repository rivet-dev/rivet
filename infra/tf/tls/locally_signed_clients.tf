locals {
	# Specify what client certs to generate
	locally_signed_client_certs = {
		nomad_client = {}
		game_guard = {}
	}
}


# MARK: Clients
resource "tls_private_key" "locally_signed_client" {
	for_each = local.locally_signed_client_certs

	algorithm = "RSA"
}

resource "tls_cert_request" "locally_signed_client" {
	for_each = local.locally_signed_client_certs

	private_key_pem = tls_private_key.locally_signed_client[each.key].private_key_pem

	subject {
		common_name  = ""
		organization = "Rivet Gaming, Inc."
	}
}

resource "tls_locally_signed_cert" "locally_signed_client" {
	for_each = local.locally_signed_client_certs

	cert_request_pem = tls_cert_request.locally_signed_client[each.key].cert_request_pem
  
	ca_key_algorithm   = "RSA"
	ca_private_key_pem = tls_private_key.root_ca.private_key_pem
	ca_cert_pem        = tls_self_signed_cert.root_ca.cert_pem
  
	validity_period_hours = 8760  # 1 year
  
	allowed_uses = [
    	"key_encipherment",
    	"digital_signature",
    	"server_auth"
  	]
}

