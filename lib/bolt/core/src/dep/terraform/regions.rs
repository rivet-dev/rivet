use anyhow::*;

use serde::Serialize;
use std::{collections::HashMap, net::Ipv4Addr};

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

	vlan: RegionVlan,
}

#[derive(Serialize, Clone)]
pub struct RegionVlan {
	address: Ipv4Addr,
	prefix_len: u8,
}

pub fn build_regions(ctx: &ProjectContext) -> Result<HashMap<String, Region>> {
	let mut regions = HashMap::new();
	for (region_id, region) in &ctx.ns().regions {
		regions.insert(
			region_id.clone(),
			Region {
				id: region.id.clone(),
				provider: region.provider.clone(),
				provider_region: region.provider_region.clone(),
				netnum: region.netnum,
				vlan: RegionVlan {
					address: net::region::vlan_ip_net().addr(),
					prefix_len: net::region::vlan_ip_net().prefix_len(),
				},
			},
		);
	}
	Ok(regions)
}
