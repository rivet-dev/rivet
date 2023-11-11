variable "namespace" {
	type = string
}

# MARK: DNS
variable "domain_main" {
	type = string
}

variable "dns_deprecated_subdomains" {
	type = bool
}

# MARK: Cloudflare
variable "cloudflare_account_id" {
	type = string
}

