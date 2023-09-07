# MARK: S3
variable "s3_buckets" {
	type = map(object({
		policy = string
    cors_allowed_origins = list(string)
	}))
}
