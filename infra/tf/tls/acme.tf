# MARK: Private key
resource "tls_private_key" "acme_account_key" {
	algorithm = "RSA"
}

# MARK: Registration
resource "acme_registration" "main" {
	account_key_pem = tls_private_key.acme_account_key.private_key_pem
	email_address = "letsencrypt@rivet.gg"
}

# MARK: Certificates
resource "acme_certificate" "rivet_gg" {
	account_key_pem = acme_registration.main.account_key_pem
	common_name = var.domain_main
	subject_alternative_names = flatten([
		"*.${var.domain_main}",

		// Add dedicated subdomains for each region
		[
			for region_id, region in var.regions:
			"*.${region_id}.${var.domain_main}"
		],
	])
	
	recursive_nameservers = ["1.1.1.1:53", "1.0.0.1:53"]

	# LetsEncrypt issues for 90 days, issue a new cert at 75 days
	min_days_remaining = 75

	# This certificate may not have been deployed yet
	revoke_certificate_on_destroy = false

	dns_challenge {
		provider = "cloudflare"

		config = {
			CF_DNS_API_TOKEN = module.secrets.values["cloudflare/terraform/auth_token"]
		}
	}
}

resource "acme_certificate" "rivet_game" {
	account_key_pem = acme_registration.main.account_key_pem
	common_name = var.domain_cdn
	subject_alternative_names = ["*.${var.domain_cdn}"]

	recursive_nameservers = ["1.1.1.1:53", "1.0.0.1:53"]

	# LetsEncrypt issues for 90 days, issue a new cert at 75 days
	min_days_remaining = 75

	# This certificate may not have been deployed yet
	revoke_certificate_on_destroy = false

	dns_challenge {
		provider = "cloudflare"

		config = {
			CF_DNS_API_TOKEN = module.secrets.values["cloudflare/terraform/auth_token"]
		}
	}
}

resource "acme_certificate" "rivet_job" {
	account_key_pem = acme_registration.main.account_key_pem
	common_name = var.domain_job
	subject_alternative_names = flatten([
		// Add dedicated subdomains for each region
		flatten([
			for region_id, region in var.regions:
			[
				"*.lobby.${region_id}.${var.domain_job}",
				"*.${region_id}.${var.domain_job}",
			]
		]),

		// Add wildcard
		"*.${var.domain_job}",
	])

	recursive_nameservers = ["1.1.1.1:53", "1.0.0.1:53"]

	# LetsEncrypt issues for 90 days, issue a new cert at 75 days
	min_days_remaining = 75

	# This certificate may not have been deployed yet
	revoke_certificate_on_destroy = false

	dns_challenge {
		provider = "cloudflare"

		config = {
			CF_DNS_API_TOKEN = module.secrets.values["cloudflare/terraform/auth_token"]
		}
	}
}

