provider "cloudflare" {
	api_token = module.secrets.values["cloudflare/terraform/auth_token"]
}

provider "kubernetes" {
	config_path = var.kubeconfig_path
}

