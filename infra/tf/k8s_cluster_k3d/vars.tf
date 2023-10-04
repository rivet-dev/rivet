variable "namespace" {
	type = string
}

variable "project_root" {
	type = string
}

variable "public_ip" {
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

variable "nomad_port" {
	type = number
	nullable = true
}

variable "api_route_port" {
	type = number
	nullable = true
}