provider "cloudflare" {
	api_token = module.secrets.values["cloudflare/terraform/auth_token"]
}
