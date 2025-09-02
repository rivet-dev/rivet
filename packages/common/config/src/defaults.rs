pub mod hosts {
	use std::net::{IpAddr, Ipv6Addr};

	pub const GUARD: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const API_PUBLIC: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const API_PEER: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const PEGBOARD_RUNNER_WS: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const PEGBOARD_GATEWAY: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
	pub const PEGBOARD_TUNNEL: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);

	pub const API_PUBLIC_LAN: &str = "::1";
	pub const PEGBOARD_RUNNER_LAN: &str = "::1";
	pub const PEGBOARD_GATEWAY_LAN: &str = "::1";
	pub const PEGBOARD_TUNNEL_LAN: &str = "::1";
}

pub mod ports {
	pub const API_PUBLIC: u16 = 6421;
	pub const API_PEER: u16 = 6422;
	pub const PEGBOARD_RUNNER_WS: u16 = 6423;
	pub const PEGBOARD_GATEWAY: u16 = 6424;
	pub const PEGBOARD_TUNNEL: u16 = 6425;
	pub const GUARD: u16 = 6420;
}
