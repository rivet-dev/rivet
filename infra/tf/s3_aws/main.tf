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
        "aws/terraform/key_id",
        "aws/terraform/key",
    ]
}

provider "aws" {
	access_key = module.secrets.values["aws/terraform/key_id"]
	secret_key = module.secrets.values["aws/terraform/key"]
	region = "us-east-1"
}
