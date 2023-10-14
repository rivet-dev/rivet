use anyhow::*;
use serde::Serialize;
use std::{
	collections::{HashMap, HashSet},
	net::Ipv4Addr,
};

use crate::context::ProjectContext;

use super::{pools::Pool, regions::Region};

#[derive(Serialize, Clone)]
pub struct Server {
	pub region_id: String,
	pub pool_id: String,
	pub version_id: String,
	pub index: usize,
	pub name: String,
	pub size: String,
	pub netnum: usize,
	pub vlan_ip: Ipv4Addr,
	pub volumes: HashMap<String, ServerVolume>,
	pub tags: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct ServerVolume {
	size: usize,
}

pub fn build_servers(
	ctx: &ProjectContext,
	regions: &HashMap<String, Region>,
	pools: &HashMap<String, Pool>,
) -> Result<HashMap<String, Server>> {
	let ns = ctx.ns_id();

	let mut servers = HashMap::<String, Server>::new();
	let mut used_netnums = HashSet::new();
	for pool in &ctx.ns().pools {
		let region_id = &pool.region;
		let pool_id = &pool.pool;
		let version_id = &pool.version;

		let _region = regions
			.get(region_id)
			.expect(&format!("missing region: {region_id}"));
		let pool_config = pools
			.get(pool_id.as_str())
			.expect(&format!("missing pool: {pool_id}"));

		// Validate netnum is within range
		assert!(
			pool.netnum > 0,
			"netnum 0 is reserved for misc services and cannot be used by a pool"
		);

		// Validate netnum is unique
		let netnum_already_used = used_netnums.insert(pool.netnum);
		assert!(
			netnum_already_used,
			"netnum {} is already used",
			pool.netnum
		);

		for i in 0..pool.count {
			let name = format!("{ns}-{region_id}-{pool_id}-{version_id}-{i}");

			let volumes = pool
				.volumes
				.iter()
				.map(|(id, volume)| (id.clone(), ServerVolume { size: volume.size }))
				.collect::<HashMap<_, _>>();

			let vlan_ip = pool_config.vlan_addr_range.clone().nth(i).unwrap();

			let server = Server {
				region_id: region_id.clone(),
				pool_id: pool_id.clone(),
				version_id: version_id.clone(),
				index: i,
				name: name.clone(),
				size: pool.size.clone(),
				netnum: pool.netnum,
				vlan_ip,
				volumes,

				// Tags that will be assigned to the servers.
				tags: vec![
					ns.to_string(),
					format!("{ns}-{region_id}"),
					format!("{ns}-{pool_id}"),
					format!("{ns}-{pool_id}-{version_id}"),
					format!("{ns}-{region_id}-{pool_id}"),
					format!("{ns}-{region_id}-{pool_id}-{version_id}"),
				],
			};

			servers.insert(name.to_string(), server);
		}
	}

	Ok(servers)
}
