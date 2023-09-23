variable "domain_main" {
	type = string
}

variable "minio_port" {
	type = string
}

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


# MARK: K8s
variable "kubeconfig_path" {
	type = string
}

variable "k8s_storage_class" {
	type = string
}

