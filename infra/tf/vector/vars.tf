variable "namespace" {
	type = string
}

variable "deploy_method_cluster" {
	type = bool
}

# MARK: ClickHouse
variable "clickhouse_enabled" {
	type = bool
}

variable "clickhouse_provider" {
	type = string
	nullable = true
}

variable "clickhouse_host" {
	type = string
}

variable "clickhouse_port_https" {
	type = string
}

# MARK: DNS
variable "tls_enabled" {
	type = bool
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
