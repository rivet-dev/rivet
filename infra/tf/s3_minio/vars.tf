variable "namespace" {
	type = string
}

# MARK: DNS
variable "domain_main" {
	type = string
}

# MARK: S3
variable "s3_buckets" {
	type = map(object({
		policy = string
    cors_allowed_origins = list(string)
	}))
}
