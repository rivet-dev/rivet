terraform {
	required_providers {
		cloudflare = {
			source = "cloudflare/cloudflare"
			version = "4.7.1"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = ["cloudflare/terraform/auth_token"]
}
