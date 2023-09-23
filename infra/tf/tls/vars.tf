variable "namespace" {
	type = string
}

# MARK: DNS
variable "domain_main" {
	type = string
}

variable "domain_cdn" {
	type = string
}

variable "domain_job" {
	type = string
}

# MARK: K8s
variable "kubeconfig_path" {
	type = string
}

# MARK: Regions
variable "regions" {
	type = map(object({
	}))
}
