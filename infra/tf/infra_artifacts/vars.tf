variable "namespace" {
	type = string
}

# MARK: S3
variable "s3_default_provider" {
	type = string
}

variable "s3_providers" {
	type = map(object({
		endpoint_internal = string
		endpoint_external = string
		region = string
	}))
}

