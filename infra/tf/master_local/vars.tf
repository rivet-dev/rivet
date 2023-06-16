variable "namespace" {
	type = string
}

# MARK: Net
variable "public_ip" {
	type = string
}

variable "local_preferred_subnets" {
	type = list(string)
}

# MARK: Net
variable "nebula_netmask" {
	type = number
}

variable "salt_master_nebula_ip" {
	type = string
}

variable "nebula_lighthouse_nebula_ip" {
	type = string
}

# MARK: Pools
variable "pools" {
	type = map(object({
		roles = list(string)
		vpc = bool
		volumes = map(object({}))
		tunnels = map(object({
			name = string
			service = string
			access_groups = list(string)
			service_tokens = list(string)
		}))
		nebula_firewall_inbound = list(object({
			proto = string
			port = string
			group = string
			host = string
		}))
	}))
}
