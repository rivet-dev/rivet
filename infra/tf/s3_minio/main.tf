terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.1.0"
		}
		kubectl = {
			source = "gavinbunney/kubectl"
			version = "1.14.0"
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

