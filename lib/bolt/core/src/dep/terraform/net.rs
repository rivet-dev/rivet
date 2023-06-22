// MARK: svc
pub mod svc {
	/// Netmask used by each region.
	///
	/// Allows for 64 regions total and 64 pools per region.
	///
	/// This is used to configure both the VPC/VLAN netmask and the IP addresses
	/// for Nebula.
	pub const REGION_NETMASK: u8 = 18;

	/// Netmask used by each pool. Pools can span multiple networks if there are
	/// a large number of nodes.
	///
	/// Allows for 64 pools per region and 254 addresses per pool (because we
	/// can't allocate the network or broadcast address).
	pub const POOL_NETMASK: u8 = 24;
}

// MARK: VPC
pub mod vpc {
	use std::net::Ipv4Addr;

	pub const SUBNET: Ipv4Addr = Ipv4Addr::new(172, 168, 0, 0);
	pub const NETMASK: u8 = 12;
}

// MARK: Nebula
pub mod nebula {
	use ipnet::Ipv4Net;
	use std::net::Ipv4Addr;

	use crate::context::ProjectContext;

	pub const SUBNET: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 0);
	pub const NETMASK: u8 = 8;

	pub const SUBNET_SVC: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 0);
	pub const NETMASK_SVC: u8 = 12;

	pub const SUBNET_JOB: Ipv4Addr = Ipv4Addr::new(10, 16, 0, 0);
	pub const NETMASK_JOB: u8 = 12;

	/// This is the IP that will be used for the local machine when running locally.
	pub const LOCAL_IP: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);

	pub fn nebula_lighthouse_nebula_ip(ctx: &ProjectContext) -> Ipv4Addr {
		match &ctx.ns().cluster.kind {
			bolt_config::ns::ClusterKind::SingleNode { .. } => LOCAL_IP,
			// Calculate IP for host
			bolt_config::ns::ClusterKind::Distributed { .. } => Ipv4Net::new(SUBNET_SVC, NETMASK)
				.unwrap()
				.hosts()
				.nth(0)
				.unwrap(),
		}
	}

	pub fn salt_master_nebula_ip(ctx: &ProjectContext) -> Ipv4Addr {
		match &ctx.ns().cluster.kind {
			bolt_config::ns::ClusterKind::SingleNode { .. } => LOCAL_IP,
			// Calculate IP for host
			bolt_config::ns::ClusterKind::Distributed { .. } => Ipv4Net::new(SUBNET_SVC, NETMASK)
				.unwrap()
				.hosts()
				.nth(9)
				.unwrap(),
		}
	}
}
