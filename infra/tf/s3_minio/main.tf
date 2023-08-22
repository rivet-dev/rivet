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
        "minio/users/root/password",
    ]
}

locals {
	s3_endpoint_internal = "http://server.minio.service.consul:9200"
	s3_endpoint_external = "https://storage.${var.domain_main}"
	# Minio defaults to us-east-1 region
	# https://github.com/minio/minio/blob/0ec722bc5430ad768a263b8464675da67330ad7c/cmd/server-main.go#L739
	s3_region = "us-east-1"
}


provider "aws" {
	region = local.s3_region
	access_key = "root"
	secret_key = module.secrets.values["minio/users/root/password"]
	skip_credentials_validation = true
	skip_metadata_api_check = true
	skip_requesting_account_id = true
	s3_use_path_style = true
	endpoints {
		s3 = local.s3_endpoint_internal
	}
}

