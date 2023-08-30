variable "namespace" {
	type = string
}

variable "deploy_method_local" {
	type = bool
}

variable "deploy_method_cluster" {
	type = bool
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

# MARK: Regions
variable "primary_region" {
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
	type = list(any)
}

# MARK: Redis
variable "redis_svcs" {
	type = map(object({
		endpoint = string
	}))
}

# MARK: S3
variable "s3_persistent_access_key_id" {
	type = string
}

variable "s3_persistent_access_key_secret" {
	type = string
	sensitive = true
}

