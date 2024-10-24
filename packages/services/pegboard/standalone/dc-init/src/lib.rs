use std::collections::HashMap;

use chirp_workflow::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Cluster {
	datacenters: HashMap<String, Datacenter>,
}

#[derive(Deserialize)]
struct Datacenter {
	datacenter_id: Uuid,
	pools: HashMap<PoolType, serde_json::Value>,
}

#[derive(Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum PoolType {
	Job,
	Gg,
	Ats,
	Pegboard,
}

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-dc-init");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-dc-init",
	)
	.await?;

	// Read config from env
	let Some(cluster_config_json) = &ctx
		.config()
		.server()?
		.rivet
		.cluster()?
		.default_cluster_config
	else {
		tracing::warn!("no cluster config set in namespace config");
		return Ok(());
	};
	let cluster_config = serde_json::from_value::<Cluster>(cluster_config_json.clone())?;

	// Find datacenter ids with pegboard pools
	let datacenter_ids = cluster_config
		.datacenters
		.iter()
		.flat_map(|(_, dc)| {
			dc.pools
				.iter()
				.any(|(pool_type, _)| matches!(pool_type, PoolType::Pegboard))
				.then_some(dc.datacenter_id)
		})
		.collect::<Vec<_>>();

	let rows = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT dc_id
		FROM UNNEST($1) AS dc(dc_id)
		WHERE EXISTS(
			SELECT 1
			FROM db_workflow.workflows
			WHERE (tags->>'datacenter_id')::UUID = dc_id
		)
		",
		datacenter_ids,
	)
	.await?;

	// Create missing datacenters
	for (datacenter_id,) in rows {
		ctx.workflow(pegboard::workflows::datacenter::Input { datacenter_id })
			.tag("datacenter_id", datacenter_id)
			.dispatch()
			.await?;
	}

	Ok(())
}
