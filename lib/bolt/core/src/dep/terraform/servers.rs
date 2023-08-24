use anyhow::*;
use ipnet::Ipv4Net;
use serde::Serialize;
use std::{
	collections::{HashMap, HashSet},
	net::Ipv4Addr,
};

use crate::context::ProjectContext;

use super::{net, pools::Pool, regions::Region};

#[derive(Serialize, Clone)]
pub struct Server {
	region_id: String,
	pool_id: String,
	version_id: String,
	index: usize,
	name: String,
	size: String,
	netnum: usize,
	volumes: HashMap<String, ServerVolume>,

	// Tags that will be assgned to the servers.
	tags: Vec<String>,

	/// IP addresses inside the VPC for all servers that belong to the VPC.
	///
	/// We add one to the hostnum in order to prevent trying to allocate the
	/// network address.
	vpc_ip: Option<Ipv4Addr>,

	/// IP addresses inside the Nebula network.
	///
	/// We add one to the hostnum in order to prevent trying to allocate the
	/// network address.
	nebula_ip: Ipv4Addr,
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

		let region = regions
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

			let server = Server {
				region_id: region_id.clone(),
				pool_id: pool_id.clone(),
				version_id: version_id.clone(),
				index: i,
				name: name.clone(),
				size: pool.size.clone(),
				netnum: pool.netnum,
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

				/// cidrhost(local.vpc_region_subnets[server.region_id], server.netnum * pow(2, 32 - local.svc_pool_netmask) + server.index + 1)
				vpc_ip: if let (true, Some(vpc_subnet)) = (pool_config.vpc, &region.vpc_subnet) {
					let n = pool.netnum * 2usize.pow(32 - net::svc::POOL_NETMASK as u32) + i + 1;
					let ip = vpc_subnet.hosts().nth(n).unwrap();

					Some(ip)
				} else {
					None
				},

				// cidrhost("${local.nebula_subnet}/${local.nebula_netmask}", var.regions[server.region_id].netnum * pow(2, 32 - local.svc_region_netmask) + server.netnum * pow(2, 32 - local.svc_pool_netmask) + server.index + 1)
				nebula_ip: {
					let subnet = Ipv4Net::new(net::nebula::SUBNET, net::nebula::NETMASK)?;
					let n = region.netnum * 2usize.pow(32 - net::svc::REGION_NETMASK as u32)
						+ pool.netnum * 2usize.pow(32 - net::svc::POOL_NETMASK as u32)
						+ i + 1;
					subnet.hosts().nth(n).unwrap()
				},
			};

			servers.insert(name.to_string(), server);
		}
	}

	Ok(servers)
}
