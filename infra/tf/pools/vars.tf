variable "namespace" {
	type = string
}

variable "deploy_method_local" {
	type = bool
}

variable "deploy_method_cluster" {
	type = bool
}

variable "server_install_scripts" {
	type = map(string)
	sensitive = true
}
