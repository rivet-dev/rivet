variable "s3_providers" {
	type = map(string, object({
		endpoint_internal = string
		endpoint_external = string
		region = string
	}))
}
