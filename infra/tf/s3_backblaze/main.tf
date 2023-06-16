terraform {
	required_providers {
		b2 = {
			source = "Backblaze/b2"
			version = "0.8.1"
		}
	}
}

module "secrets" {
    source = "../modules/secrets"

    keys = [
        "b2/terraform/key_id",
        "b2/terraform/key",
    ]
}

provider "b2" {
	application_key_id = module.secrets.values["b2/terraform/key_id"]
	application_key = module.secrets.values["b2/terraform/key"]
}

locals {
	s3_endpoint = data.b2_account_info.main.s3_api_url
	# See region information here:
	# https://help.backblaze.com/hc/en-us/articles/360047425453-Getting-Started-with-the-S3-Compatible-API
	s3_region = split(".", data.b2_account_info.main.s3_api_url)[1]
}

data "b2_account_info" "main" {}
