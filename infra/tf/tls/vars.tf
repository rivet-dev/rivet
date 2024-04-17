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

# MARK: S3
variable "s3_providers" {
	type = map(object({
		endpoint_internal = string
		endpoint_external = string
		region = string
	}))
}
