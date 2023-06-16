# MARK: Connection
variable "install_local" {
	type = bool
}

variable "install_remote" {
	type = bool
}

variable "host" {
	type = string
	nullable = true
	default = null
}

variable "user" {
	type = string
	nullable = true
	default = null
}

variable "private_key_openssh" {
	type = string
	sensitive = true
	nullable = true
	default = null
}

# MARK: Nebula
variable "nebula_ca_cert" {
	type = string
}

variable "nebula_ca_key" {
	type = string
	sensitive = true
}

# https://github.com/slackhq/nebula/releases
variable "nebula_version" {
	type = string
	default = "1.7.2"
}

variable "nebula_name" {
	type = string
}

variable "nebula_ip" {
	type = string
}

variable "nebula_netmask" {
	type = number
}

variable "nebula_groups" {
	type = list(string)
}

variable "is_lighthouse" {
	type = bool
	default = false
}

variable "static_host_map" {
	type = map(list(string))
	sensitive = true
}

variable "lighthouse_hosts" {
	type = list(string)
}

variable "preferred_ranges" {
	type = list(string)
}

variable "firewall" {
	type = any
}

