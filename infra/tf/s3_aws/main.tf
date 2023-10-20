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

provider "aws" {
	access_key = module.secrets.values["s3/aws/terraform/key_id"]
	secret_key = module.secrets.values["s3/aws/terraform/key"]
	region = "us-east-1"

	default_tags {
		tags = {
			Namespace = var.namespace
		}
	}
}

