pub mod region {
	use std::net::Ipv4Addr;

	pub const VLAN_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 0);
	pub const VLAN_PREFIX_LEN: u8 = 16;
}

pub mod gg {
	use std::net::Ipv4Addr;

	pub const VLAN_ADDR: Ipv4Addr = super::region::VLAN_ADDR;
	pub const VLAN_PREFIX_LEN: u8 = 26;
}

pub mod ats {
	use std::net::Ipv4Addr;

	pub const VLAN_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 64);
	pub const VLAN_PREFIX_LEN: u8 = 26;
}

// 10.0.64-10.0.4.0 reserved for more services

pub mod job {
	use std::net::Ipv4Addr;

	pub const VLAN_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 4, 0);
	pub const VLAN_PREFIX_LEN: u8 = 16;
}
