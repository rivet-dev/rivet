variable "install_local" {
	type = bool
}

variable "install_remote" {
	type = bool
}

variable "host" {
	type = string
	default = ""
}

variable "user" {
	type = string
	default = ""
}

variable "private_key" {
	type = string
	sensitive = true
	default = ""
}

variable "salt_master_name" {
	type = string
}

