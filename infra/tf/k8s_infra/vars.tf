variable "namespace" {
	type = string
}

variable "deploy_method_cluster" {
	type = bool
}

variable "public_ip" {
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
variable "redis_dbs" {
	type = map(object({
		endpoint = string
	}))
}

# MARK: Regions
variable "regions" {
	type = map(any)
}

# MARK: K8s
variable "kubeconfig_path" {
	type = string
}

variable "k8s_storage_class" {
	type = string
}

variable "k8s_health_port" {
	type = number
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

variable "s3_buckets" {
	type = map(any)
}

# MARK: Rivet
variable "cdn_cache_size_gb" {
	type = number
}
