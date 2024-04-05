pub struct FirewallRule {
	pub label: String,
	pub ports: String,
	pub protocol: String,
	pub inbound_ipv4_cidr: Vec<String>,
	pub inbound_ipv6_cidr: Vec<String>,
}

pub fn default_firewall() -> FirewallRule {
	FirewallRule {
		label: "ssh".into(),
		ports: "22".into(),
		protocol: "tcp".into(),
		inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
		inbound_ipv6_cidr: vec!["::/0".into()],
	}
}

pub mod region {
	use std::net::Ipv4Addr;

	use ipnet::Ipv4Net;

	pub fn vlan_ip_net() -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 16).unwrap()
	}
}

pub mod gg {
	use std::net::Ipv4Addr;

	use ipnet::{Ipv4AddrRange, Ipv4Net};

	use super::{default_firewall, FirewallRule, job};

	pub fn vlan_ip_net() -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 26).unwrap()
	}

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		vlan_ip_net().hosts()
	}

	pub fn firewall() -> Vec<FirewallRule> {
		vec![
			default_firewall(),
			// HTTP(S)
			FirewallRule {
				label: "http-tcp".into(),
				ports: "80".into(),
				protocol: "tcp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			FirewallRule {
				label: "http-udp".into(),
				ports: "80".into(),
				protocol: "udp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			FirewallRule {
				label: "https-tcp".into(),
				ports: "443".into(),
				protocol: "tcp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			FirewallRule {
				label: "https-udp".into(),
				ports: "443".into(),
				protocol: "udp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			// Dynamic TCP
			FirewallRule {
				label: "dynamic-tcp".into(),
				ports: format!("{}-{}", job::MIN_INGRESS_PORT_TCP, job::MAX_INGRESS_PORT_TCP),
				protocol: "tcp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			// Dynamic UDP
			FirewallRule {
				label: "dynamic-udp".into(),
				ports: format!("{}-{}", job::MIN_INGRESS_PORT_UDP, job::MAX_INGRESS_PORT_UDP),
				protocol: "udp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
		]
	}
}

pub mod ats {
	use std::net::Ipv4Addr;

	use ipnet::{Ipv4AddrRange, Ipv4Net};

	use super::{default_firewall, FirewallRule};

	pub fn vlan_ip_net() -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 64), 26).unwrap()
	}

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		vlan_ip_net().hosts()
	}

	pub fn firewall() -> Vec<FirewallRule> {
		vec![default_firewall()]
	}
}

// 10.0.64-10.0.4.0 reserved for more services

pub mod job {
	use std::net::Ipv4Addr;

	use ipnet::Ipv4AddrRange;

	use super::{default_firewall, FirewallRule};

	// Port ranges for the load balancer hosts
	// 20000-26000 are for traffic from gg on LAN
	// 26000-31999 is for host networking only
	pub const MIN_INGRESS_PORT_TCP: u16 = 20000;
	pub const MIN_HOST_PORT_TCP: u16 = 26000;
	pub const MAX_INGRESS_PORT_TCP: u16 = 31999;
	pub const MIN_INGRESS_PORT_UDP: u16 = 20000;
	pub const MIN_HOST_PORT_UDP: u16 = 26000;
	pub const MAX_INGRESS_PORT_UDP: u16 = 31999;

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		Ipv4AddrRange::new(Ipv4Addr::new(10, 0, 4, 1), Ipv4Addr::new(10, 0, 255, 254))
	}

	pub fn firewall() -> Vec<FirewallRule> {
		vec![
			default_firewall(),
			// Ports available to Nomad jobs using the host network
			FirewallRule {
				label: "nomad-host-tcp".into(),
				ports: format!("{}-{}", MIN_HOST_PORT_TCP, MAX_INGRESS_PORT_TCP),
				protocol: "tcp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
			FirewallRule {
				label: "nomad-host-udp".into(),
				ports: format!("{}-{}", MIN_HOST_PORT_UDP, MAX_INGRESS_PORT_UDP),
				protocol: "udp".into(),
				inbound_ipv4_cidr: vec!["0.0.0.0/0".into()],
				inbound_ipv6_cidr: vec!["::/0".into()],
			},
		]
	}
}
