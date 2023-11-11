provider "cloudflare" {
	api_token = module.secrets.values["cloudflare/terraform/auth_token"]
}

provider "acme" {
	# See https://letsencrypt.org/docs/acme-protocol-updates/#api-endpoints
	#
	# You may need to change the private key when changing the server URL. See
	# https://github.com/vancluever/terraform-provider-acme/issues/110
	server_url = "https://acme-v02.api.letsencrypt.org/directory"
	# server_url = "https://acme-staging-v02.api.letsencrypt.org/directory"
}

provider "kubernetes" {
	config_path = var.kubeconfig_path
}

