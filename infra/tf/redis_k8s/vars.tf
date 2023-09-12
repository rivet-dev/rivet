variable "deploy_method_cluster" {
	type = bool
}

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

# MARK: Docker
variable "authenticate_all_docker_hub_pulls" {
	type = bool
}
