terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.1.0"
		}
	}
}

locals {
	s3_provider = var.s3_providers[var.s3_default_provider]
}


module "secrets" {
    source = "../modules/secrets"

    keys = [
        "s3/${var.s3_default_provider}/terraform/key_id",
        "s3/${var.s3_default_provider}/terraform/key",
    ]
}

