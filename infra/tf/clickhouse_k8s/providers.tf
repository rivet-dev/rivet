provider "kubernetes" {
	config_path = var.kubeconfig_path
}

provider "helm" {
	kubernetes {
		config_path = var.kubeconfig_path
	}
}

