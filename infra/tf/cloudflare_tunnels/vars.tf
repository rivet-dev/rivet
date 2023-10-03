variable "namespace" {
	type = string
}

# MARK: DNS
variable "domain_main" {
	type = string
}

# MARK: Cloudflare
variable "cloudflare_account_id" {
	type = string
}

variable "cloudflare_zone_id_rivet_gg" {
	type = string
}

# MARK: Pools
variable "pools" {
	type = map(object({
		tunnels = map(object({
			name = string
			service = string
			access_groups = list(string)
			service_tokens = list(string)
		}))
	}))
}

variable "kubeconfig_path" {
	type = string
}
