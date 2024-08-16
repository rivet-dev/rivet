variable "namespace" {
	type = string
}

variable "k3d_use_local_repo" {
	type = bool
}

variable "project_root" {
	type = string
}

variable "cargo_target_dir" {
	type = string
}

variable "volumes_dir" {
	type = string
}

variable "api_http_port" {
	type = number
}

variable "api_https_port" {
	type = number
	nullable = true
}

variable "minio_port" {
	type = number
	nullable = true
}

variable "tunnel_port" {
	type = number
}

