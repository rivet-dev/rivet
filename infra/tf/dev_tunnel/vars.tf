variable "namespace" {
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

