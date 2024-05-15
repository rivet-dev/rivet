variable "namespace" {
	type = string
}

variable "kubeconfig_path" {
	type = string
}

variable "cockroachdb_spend_limit" {
	type = number
}

variable "cockroachdb_request_unit_limit" {
	type = string
}

variable "cockroachdb_storage_limit" {
	type = string
}

variable "prometheus_enabled" {
	type = bool
}
