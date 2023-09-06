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
