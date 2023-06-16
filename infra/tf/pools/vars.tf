variable "namespace" {
	type = string
}

variable "deploy_method_local" {
	type = bool
}

variable "deploy_method_cluster" {
	type = bool
}

# MARK: Net
variable "svc_region_netmask" {
	type = string
}

variable "nebula_netmask" {
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
	type = map(object({
		id = string
		provider = string
		provider_region = string
		netnum = number
		supports_vlan = bool
		preferred_subnets = list(string)
	}))
}

# MARK: Pools
variable "pools" {
	type = map(object({
		roles = list(string)
		vpc = bool
		tunnels = map(object({
			name = string
			service = string
			access_groups = list(string)
			service_tokens = list(string)
		}))
		firewall_inbound = list(object({
			label = string
			ports = string
			protocol = string
			inbound_ipv4_cidr = list(string)
			inbound_ipv6_cidr = list(string)
		}))
		nebula_firewall_inbound = list(object({
			proto = string
			port = string
			group = string
			host = string
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
		volumes = map(object({
			size = number
		}))
		tags = list(string)
		vpc_ip = string
		nebula_ip = string
	}))
}
