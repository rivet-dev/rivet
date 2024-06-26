# MARK: Better Uptime
variable "better_uptime_groups" {
	type = list(object({
		id = string
		name = string
		monitors = list(object({
			id = string
			url = string
			public_name = string
			verify_ssl = optional(bool)
		}))
	}))
}

variable "better_uptime" {
	type = object({
		company_name = string
		company_url = string
		company_subdomain = string
	})
}

variable "better_uptime_notify" {
	type = bool
}

