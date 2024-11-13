use chirp_workflow::prelude::*;

// TODO: This is not idempotent.
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
	let cluster_configs = &ctx.config().server()?.rivet.clusters();

	for cluster_config in cluster_configs.values() {
		// Find datacenter ids with pegboard pools
		let datacenter_ids = cluster_config
			.datacenters
			.values()
			.map(|x| x.id)
			.collect::<Vec<_>>();

		let rows = sql_fetch_all!(
			[ctx, (Uuid,)]
			"
			SELECT dc_id
			FROM UNNEST($1) AS dc(dc_id)
			WHERE NOT EXISTS(
				SELECT 1
				FROM db_workflow.workflows
				WHERE
					workflow_name = 'pegboard_datacenter' AND
					(tags->>'datacenter_id')::UUID = dc_id
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
	}

	Ok(())
}
