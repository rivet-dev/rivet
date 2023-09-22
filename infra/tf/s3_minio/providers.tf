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
		s3 = "https://minio.${var.domain_main}"
	}
}

provider "kubernetes" {
	config_path = var.kubeconfig_path
}

provider "helm" {
	kubernetes {
		config_path = var.kubeconfig_path
	}
}

provider "kubectl" {
	config_path = var.kubeconfig_path
}

