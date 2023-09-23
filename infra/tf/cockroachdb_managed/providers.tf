provider "cockroach" {
	apikey = module.secrets.values["cockroachdb_cloud/api_key"]
}

provider "kubernetes" {
	config_path = var.kubeconfig_path
}
