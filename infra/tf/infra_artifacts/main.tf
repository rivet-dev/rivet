terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.1.0"
		}
	}
}

module "secrets" {
    source = "../modules/secrets"

    keys = [
        "s3/aws/terraform/key_id",
        "s3/aws/terraform/key",
    ]
}

