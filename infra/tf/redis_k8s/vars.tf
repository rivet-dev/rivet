variable "kubeconfig_path" {
	type = string
}

variable "k8s_storage_class" {
	type = string
}

variable "redis_dbs" {
	type = map(object({
		endpoint = string
	}))
}

