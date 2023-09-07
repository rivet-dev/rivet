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
        "s3/minio/root/key_id",
        "s3/minio/root/key",
    ]
}

provider "aws" {
	# Minio defaults to us-east-1 region
	# https://github.com/minio/minio/blob/0ec722bc5430ad768a263b8464675da67330ad7c/cmd/server-main.go#L739
	region = "us-east-1"
	access_key = module.secrets.values["s3/minio/root/key_id"]
	secret_key = module.secrets.values["s3/minio/root/key"]
	skip_credentials_validation = true
	skip_metadata_api_check = true
	skip_requesting_account_id = true
	s3_use_path_style = true
	endpoints {
		s3 = "http://127.0.0.1:9200"
	}
}
