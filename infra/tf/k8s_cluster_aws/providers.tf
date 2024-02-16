provider "aws" {
	region = local.region
	default_tags {
		tags = {
			Namespace = var.namespace
		}
	}
}

# NOTE: This provider setup differs from the ones used everywhere else because this is a cluster service that
# requires the kubernetes provisioner before the cluster itself is created. All other terraform configs can
# simply read the kubeconfig file, but for this config it does not yet exist because this is config that
# creates it.
provider "kubernetes" {
	host = module.eks.cluster_endpoint
	cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = ["--region", "us-east-1", "eks", "get-token", "--cluster-name", module.eks.cluster_name, "--output", "json", "--role", "arn:aws:iam::717589162638:role/rivet-staging2-Admin"]
	}
}

provider "helm" {
	kubernetes {
		host = module.eks.cluster_endpoint
		cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

		exec {
			api_version = "client.authentication.k8s.io/v1beta1"
			command = "aws"
			args = ["--region", "us-east-1", "eks", "get-token", "--cluster-name", module.eks.cluster_name, "--output", "json", "--role", "arn:aws:iam::717589162638:role/rivet-staging2-Admin"]
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
		args = ["--region", "us-east-1", "eks", "get-token", "--cluster-name", module.eks.cluster_name, "--output", "json", "--role", "arn:aws:iam::717589162638:role/rivet-staging2-Admin"]
	}
}
