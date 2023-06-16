use anyhow::*;
use ipnet::Ipv4Net;
use serde::Serialize;
use std::collections::HashMap;

use crate::context::ProjectContext;

use super::net;

#[derive(Serialize, Clone)]
pub struct Region {
	// Unique UUID that represents this region.
	id: String,

	// Name of the server provider to use.
	//
	// Current options:
	// * digitalocean
	// * linode
	provider: String,

	// This is the name of the provider's region.
	// * DigitalOcean: https://docs.digitalocean.com/products/platform/availability-matrix/
	// * Linode: linode-cli regions list
	provider_region: String,

	pub netnum: usize,

	supports_vlan: bool,

	/// VPC subnet for this region.
	///
	/// Only exists if `supports_vlan` is true.
	pub vpc_subnet: Option<Ipv4Net>,

	/// Private subnets in order or priority.
	///
	/// This includes `vpc_subnet` if exists.
	pub preferred_subnets: Vec<String>,
}

pub fn build_regions(ctx: &ProjectContext) -> Result<HashMap<String, Region>> {
	let mut regions = HashMap::new();
	for (region_id, region) in &ctx.ns().regions {
		// Calculate VPC subnet
		// cidrsubnet("${local.vpc_subnet}/${local.vpc_netmask}", local.svc_region_netmask - local.vpc_netmask, region.netnum)
		let vpc_subnet = if region.supports_vlan {
			Some(
				Ipv4Net::new(net::vpc::SUBNET, net::vpc::NETMASK)?
					.subnets(net::svc::REGION_NETMASK)?
					.nth(region.netnum)
					.unwrap(),
			)
		} else {
			None
		};

		// Generate subnets
		let mut preferred_subnets = region.preferred_subnets.clone();
		if let Some(vpc_subnet) = vpc_subnet {
			// Add VPC as top priority
			preferred_subnets.insert(0, vpc_subnet.to_string());
		}

		regions.insert(
			region_id.clone(),
			Region {
				id: region.id.clone(),
				provider: region.provider.clone(),
				provider_region: region.provider_region.clone(),
				netnum: region.netnum,
				supports_vlan: region.supports_vlan,
				vpc_subnet,
				preferred_subnets,
			},
		);
	}
	Ok(regions)
}
