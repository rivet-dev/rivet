terraform {
	required_providers {
		kubectl = {
			source = "gavinbunney/kubectl"
			version = "1.14.0"
		}
	}
}

provider "helm" {
	kubernetes {
		config_path = "~/.kube/config"
	}
}

provider "kubernetes" {
	config_path = "~/.kube/config"
}

provider "kubectl" {
	config_path = "~/.kube/config"
}

module "secrets" {
	source = "../modules/secrets"

	keys = flatten([
		var.authenticate_all_docker_hub_pulls ? [
			"docker/docker_io/username",
			"docker/docker_io/password",
		] : [],
	])
}
