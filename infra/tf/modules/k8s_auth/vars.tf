variable "namespaces" {
	type = set(string)
}

variable "authenticate_all_docker_hub_pulls" {
	type = bool
	default = false
}

variable "deploy_method_cluster" {
	type = bool
}
