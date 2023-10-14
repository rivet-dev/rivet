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

	pub fn vlan_ip_net() -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 0), 26).unwrap()
	}

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		vlan_ip_net().hosts()
	}
}

pub mod ats {
	use std::net::Ipv4Addr;

	use ipnet::{Ipv4AddrRange, Ipv4Net};

	pub fn vlan_ip_net() -> Ipv4Net {
		Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 64), 26).unwrap()
	}

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		vlan_ip_net().hosts()
	}
}

// 10.0.64-10.0.4.0 reserved for more services

pub mod job {
	use std::net::Ipv4Addr;

	use ipnet::Ipv4AddrRange;

	pub fn vlan_addr_range() -> Ipv4AddrRange {
		Ipv4AddrRange::new(Ipv4Addr::new(10, 0, 4, 1), Ipv4Addr::new(10, 0, 255, 254))
	}
}
