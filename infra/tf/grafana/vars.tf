variable "namespace" {
	type = string
}

# MARK: Services
variable "services" {
	type = map(object({
		count = number
		resources = object({
			cpu = number
			memory = number
		})
	}))
}

# MARK: K8s
variable "kubeconfig_path" {
	type = string
}

variable "limit_resources" {
	type = bool
}
