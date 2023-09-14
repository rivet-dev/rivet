variable "namespace" {
	type = string
}

variable "private_key_openssh" {
	type = string
	sensitive = true
}

# MARK: Server
variable "region" {
	type = object({
		provider = string
		provider_region = string
		vlan = object({
			address = string
			prefix_len = number
		})
	})
}

variable "size" {
    type = string
}

variable "label" {
    type = string
}

variable "tags" {
    type = list(string)
}

variable "backup" {
    type = bool
    default = false
}

variable "volumes" {
    type = map(object({
		size = number
	}))
	default = {}
}

variable "firewall_inbound" {
	type = list(object({
		label = string
		ports = string
		protocol = string
		inbound_ipv4_cidr = list(string)
		inbound_ipv6_cidr = list(string)
	}))
	default = []
}
