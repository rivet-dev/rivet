# MARK: Tunnel
variable "name" {
	type = string
}

# MARK: Cloudfalre
variable "cloudflare_account_id" {
	type = string
}

# MARK: cloudflared
variable "ingress" {
	type = map(object({
		app_name = string
		cloudflare_zone_id = string
		access_groups = list(string)
		service_tokens = list(string)
		service = string
		app_launcher = optional(bool)
		no_app = optional(bool)
	}))
}

