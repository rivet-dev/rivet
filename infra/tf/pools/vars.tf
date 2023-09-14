variable "namespace" {
	type = string
}

variable "deploy_method_local" {
	type = bool
}

variable "deploy_method_cluster" {
	type = bool
}

# MARK: Regions
variable "primary_region" {
	type = string
}

variable "regions" {
	type = map(object({
		id = string
		provider = string
		provider_region = string
		vlan = object({
			address = string
			prefix_len = number
		})
	}))
}

# MARK: Pools
variable "pools" {
	type = map(object({
		firewall_inbound = list(object({
			label = string
			ports = string
			protocol = string
			inbound_ipv4_cidr = list(string)
			inbound_ipv6_cidr = list(string)
		}))
	}))
}

# MARK: Servers
variable "servers" {
	type = map(object({
		region_id = string
		pool_id = string
		version_id = string
		index = number
		name = string
		size = string
		netnum = number
		vlan_ip = string
		volumes = map(object({
			size = number
		}))
		tags = list(string)
	}))
}
