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

# MARK: Services
variable "services" {
	type = map(object({
		count = number
		resources = object({
			cpu = number
			cpu_cores = number
			memory = number
		})
	}))
}

# MARK: Docker
variable "authenticate_all_docker_hub_pulls" {
	type = bool
}

# MARK: Imagor
variable "imagor_presets" {
	type = any
}

# MARK: Redis
variable "redis_svcs" {
	type = map(object({
		endpoint = string
	}))
}

# MARK: Regions
variable "regions" {
	type = map(any)
}

# MARK: K8s
variable "k8s_health_port" {
	type = number
}

# MARK: S3
variable "s3_persistent_access_key_id" {
	type = string
}

variable "s3_persistent_access_key_secret" {
	type = string
	sensitive = true
}

variable "has_minio" {
	type = bool
}
