terraform {
	required_providers {
		cloudflare = {
			source = "cloudflare/cloudflare"
			version = "4.7.1"
		}

		acme = {
			source  = "vancluever/acme"
			version = "2.10.0"
		}

		tls = {
			source = "hashicorp/tls"
				version = "3.4.0"
		}
	}
}

module "secrets" {
    source = "../modules/secrets"

    keys = [
        "cloudflare/terraform/auth_token",
    ]
}

locals {
	has_minio = can(var.s3_providers["minio"])
}

