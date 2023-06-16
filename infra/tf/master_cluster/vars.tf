variable "namespace" {
	type = string
}

# MARK: Net
variable "nebula_netmask" {
	type = string
}

variable "nebula_lighthouse_nebula_ip" {
	type = string
}

variable "salt_master_nebula_ip" {
	type = string
}

# MARK: Regions
variable "primary_region" {
	type = string
}

variable "regions" {
	type = map(any)
}

# MARK: Salt Master
variable "salt_master_size" {
	type = string
}

# MARK: Nebula Lighthouse
variable "nebula_lighthouse_size" {
	type = string
}
