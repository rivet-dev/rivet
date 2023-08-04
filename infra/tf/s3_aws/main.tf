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
	region = local.s3_region
}

locals {
	s3_region = "us-east-1"
	s3_endpoint = "https://s3.${local.s3_region}.amazonaws.com"
	# See region information here:
	# https://docs.aws.amazon.com/AmazonS3/latest/userguide/RESTAPI.html
}
