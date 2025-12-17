pub mod hosts {
	use std::net::{IpAddr, Ipv6Addr};

	pub const GUARD: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const API_PEER: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const METRICS: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
}

pub mod ports {
	pub const GUARD: u16 = 6420;
	pub const API_PEER: u16 = 6421;

	pub const METRICS: u16 = 6430;
}
