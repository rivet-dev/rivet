use std::{collections::HashMap, str::FromStr};

use chirp_workflow::prelude::*;

use crate::types::{Filter, PoolType, Server};

#[derive(Debug)]
pub struct Input {
	pub datacenter_id: Uuid,
	pub pool_types: Vec<PoolType>,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

/// Wrapper around server::list with very short cache.
#[operation]
pub async fn cluster_datacenter_server_discovery(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let cache_keys = if input.pool_types.is_empty() {
		vec![(input.datacenter_id, "all".to_string())]
	} else {
		input
			.pool_types
			.iter()
			.map(|pool| (input.datacenter_id, pool.to_string()))
			.collect()
	};

	let servers = ctx
		.cache()
		.ttl(5000)
		.fetch_all_json("cluster.datacenter.service_discovery", cache_keys, {
			let ctx = ctx.clone();
			move |mut cache, keys| {
				let ctx = ctx.clone();
				async move {
					let pools = keys
						.into_iter()
						.filter(|(_, pool)| pool != "all")
						.map(|(_, pool)| PoolType::from_str(&pool))
						.collect::<GlobalResult<Vec<_>>>()?;

					let servers_res = ctx
						.op(crate::ops::server::list::Input {
							filter: Filter {
								datacenter_ids: Some(vec![input.datacenter_id]),
								pool_types: (!pools.is_empty()).then(|| pools),
								..Default::default()
							},
							include_destroyed: false,
							exclude_installing: true,
							exclude_draining: true,
							exclude_no_vlan: true,
						})
						.await?;

					let mut servers_by_pool_type =
						HashMap::with_capacity(servers_res.servers.len());

					for server in servers_res.servers {
						servers_by_pool_type
							.entry(server.pool_type)
							.or_insert_with(Vec::new)
							.push(server);
					}

					for (pool_type, servers) in servers_by_pool_type {
						cache.resolve(&(input.datacenter_id, pool_type.to_string()), servers);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(Output {
		servers: servers.into_iter().flatten().collect(),
	})
}
