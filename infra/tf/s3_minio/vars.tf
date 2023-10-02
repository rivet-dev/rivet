# MARK: S3
variable "s3_buckets" {
	type = map(object({
		policy = string
    cors_allowed_origins = list(string)
	}))
}

variable "s3_providers" {
	type = map(object({
		endpoint_internal = string
		endpoint_external = string
		region = string
	}))
}


