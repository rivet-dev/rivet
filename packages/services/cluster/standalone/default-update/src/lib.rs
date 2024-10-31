use chirp_workflow::prelude::*;

pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	use_autoscaler: bool,
) -> GlobalResult<()> {
	// TODO: When running bolt up, this service gets created first before `cluster-worker` so the messages
	// sent from here are received but effectively forgotten because `cluster-worker` gets restarted
	// immediately afterwards. This server will be replaced with a bolt infra step
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;

	start_inner(config, pools, use_autoscaler).await
}

#[tracing::instrument(skip_all)]
pub async fn start_inner(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	use_autoscaler: bool,
) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-default-update");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-default-update",
	)
	.await?;

	// Read config from env
	let Ok(config) = &ctx
		.config()
		.server()?
		.rivet
		.cluster()
	else {
		tracing::warn!("no cluster config set in namespace config");
		return Ok(());
	};

	// HACK: When deploying both monolith worker and this service for the first time, there is a race
	// condition which might result in the message being published from here but not caught by
	// monolith-worker, resulting in nothing happening.
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	let cluster_id = cluster::util::default_cluster_id();

	let (cluster_res, datacenter_list_res) = tokio::try_join!(
		// Check if cluster already exists
		ctx.op(cluster::ops::get::Input {
			cluster_ids: vec![cluster_id],
		}),
		ctx.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_id],
		}),
	)?;

	// Get all datacenters
	let cluster = unwrap!(datacenter_list_res.clusters.first());
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: cluster.datacenter_ids.clone(),
		})
		.await?;

	if cluster_res.clusters.is_empty() {
		tracing::warn!("creating default cluster");

		ctx.workflow(cluster::workflows::cluster::Input {
			cluster_id,
			name_id: config.name_id.clone(),
			owner_team_id: None,
		})
		.tag("cluster_id", cluster_id)
		.dispatch()
		.await?;
	}

	for existing_datacenter in &datacenters_res.datacenters {
		if !config
			.datacenters
			.iter()
			.any(|(_, dc)| dc.datacenter_id == existing_datacenter.datacenter_id)
		{
			// TODO: Delete datacenters
		}
	}

	for (name_id, datacenter) in &config.datacenters {
		let existing_datacenter = datacenters_res
			.datacenters
			.iter()
			.any(|dc| dc.datacenter_id == datacenter.datacenter_id);

		// Update existing datacenter
		if existing_datacenter {
			let new_pools = datacenter
				.pools
				.iter()
				.map(|(pool_type, pool)| {
					let desired_count = if use_autoscaler {
						None
					} else {
						Some(pool.desired_count)
					};

					cluster::types::PoolUpdate {
						pool_type: (*pool_type).into(),
						hardware: pool
							.hardware
							.iter()
							.cloned()
							.map(Into::into)
							.collect::<Vec<_>>(),
						desired_count,
						min_count: Some(pool.min_count),
						max_count: Some(pool.max_count),
						drain_timeout: Some(pool.drain_timeout),
					}
				})
				.collect::<Vec<_>>();

			ctx.signal(cluster::workflows::datacenter::Update {
				pools: new_pools,
				prebakes_enabled: Some(datacenter.prebakes_enabled),
			})
			.tag("datacenter_id", datacenter.datacenter_id)
			.send()
			.await?;
		}
		// Create new datacenter
		else {
			ctx.signal(cluster::workflows::cluster::DatacenterCreate {
				datacenter_id: datacenter.datacenter_id,
				name_id: name_id.clone(),
				display_name: datacenter.display_name.clone(),

				provider: datacenter.provider.into(),
				provider_datacenter_id: datacenter.provider_datacenter_name.clone(),
				provider_api_token: None,

				pools: datacenter
					.pools
					.iter()
					.map(|(pool_type, pool)| cluster::types::Pool {
						pool_type: (*pool_type).into(),
						hardware: pool
							.hardware
							.iter()
							.cloned()
							.map(Into::into)
							.collect::<Vec<_>>(),
						desired_count: pool.desired_count,
						min_count: pool.min_count,
						max_count: pool.max_count,
						drain_timeout: pool.drain_timeout,
					})
					.collect::<Vec<_>>(),

				build_delivery_method: datacenter.build_delivery_method.into(),
				prebakes_enabled: datacenter.prebakes_enabled,
			})
			.tag("cluster_id", cluster_id)
			.send()
			.await?;
		}
	}

	Ok(())
}
