variable "namespace" {
	type = string
}

variable "deploy_method_cluster" {
	type = bool
}

variable "public_ip" {
	type = string
	nullable = true
	default = null
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

variable "domain_main_api" {
	type = string
	nullable = true
}

variable "dns_deprecated_subdomains" {
	type = bool
}

variable "tls_enabled" {
	type = bool
}

variable "minio_port" {
	type = string
	nullable = true
}

# MARK: Services
variable "services" {
	type = map(object({
		count = number
		resources = object({
			cpu = number
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

variable "imagor_cors_allowed_origins" {
	type = list(string)
}

# MARK: CockroachDB
variable "cockroachdb_provider" {
	type = string
}

# MARK: ClickHouse
variable "clickhouse_provider" {
	type = string
}

# MARK: Redis
variable "redis_replicas" {
	type = number
}

variable "redis_provider" {
	type = string
}

variable "redis_dbs" {
	type = map(object({
		persistent =  bool
	}))
}

# MARK: K8s
variable "kubeconfig_path" {
	type = string
}

variable "k8s_storage_class" {
	type = string
}

variable "limit_resources" {
	type = bool
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
