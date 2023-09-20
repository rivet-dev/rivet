provider "aws" {
	region = local.region
	default_tags {
		tags = {
			Namespace = var.namespace
		}
	}
}

provider "kubernetes" {
	host = module.eks.cluster_endpoint
	cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
	}
}

provider "helm" {
	kubernetes {
		host = module.eks.cluster_endpoint
		cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

		exec {
			api_version = "client.authentication.k8s.io/v1beta1"
			command = "aws"
			args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
		}
	}
}

provider "kubectl" {
	apply_retry_count = 5
	host = module.eks.cluster_endpoint
	cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)
	load_config_file = false

	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
	}
}
