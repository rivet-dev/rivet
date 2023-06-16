variable "namespace" {
	type = string
}

variable "install_local" {
	type = bool
}

variable "install_remote" {
	type = bool
}

# MARK: Salt Master
variable "master_host" {
	type = string
	nullable = true
	default = null
}

variable "master_user" {
	type = string
	nullable = true
	default = null
}

variable "master_private_key" {
	type = string
	sensitive = true
	nullable = true
	default = null
}

variable "master_nebula_ip" {
	type = string
}

# MARK: Salt Minion
# Will skip installing Salt.
variable "skip_install" {
	type = bool
	default = false
}

# If the minion is running locally or on a remote machine.
variable "local_minion" {
	type = bool
	default = false
}

variable "minion_host" {
	type = string
	nullable = true
	default = null
}

variable "minion_user" {
	type = string
	nullable = true
	default = null
}

variable "minion_private_key" {
	type = string
	sensitive = true
	nullable = true
	default = null
}

variable "minion_server_id" {
	type = string
}

# MARK: Salt Grain
variable "roles" {
	type = list(string)
}

variable "region" {
	type = object({
		provider = string
		provider_region = string
	})
}

variable "server" {
	type = object({
		region_id = string
		pool_id = string
		version_id = string
		index = number
		name = string
		size = string
	})
}

variable "vpc" {
	type = object({
		ips = list(string)
	})
}

variable "nebula" {
	type = object({
		ipv4 = string
	})
}

variable "volumes" {
	type = map(object({
		size = number
		mount = bool
	}))
}
