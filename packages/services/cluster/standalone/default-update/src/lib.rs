use chirp_workflow::prelude::*;
use rivet_config::config;

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

	let rivet_config = &ctx.config().server()?.rivet;

	let cluster_configs = rivet_config.clusters();

	for (cluster_slug, cluster) in cluster_configs.iter() {
		upsert_cluster(&ctx, use_autoscaler, cluster_slug, cluster).await?;
	}

	Ok(())
}

/// Creates or updates an existing cluster.
async fn upsert_cluster(
	ctx: &StandaloneCtx,
	use_autoscaler: bool,
	cluster_slug: &str,
	cluster_config: &config::rivet::Cluster,
) -> GlobalResult<()> {
	// Fetch cluster data
	let cluster_res = ctx
		.op(cluster::ops::get::Input {
			cluster_ids: vec![cluster_config.id],
		})
		.await?;

	// Create cluster if needed
	let cluster = if let Some(cluster) = cluster_res.clusters.first() {
		// Validate the cluster config has not changed
		ensure_eq!(
			cluster_slug,
			cluster.name_id,
			"cluster id does not match config"
		);

		cluster.clone()
	} else {
		tracing::debug!("creating default cluster");

		let mut create_sub = ctx
			.subscribe::<cluster::workflows::cluster::CreateComplete>((
				"cluster_id",
				cluster_config.id,
			))
			.await?;
		ctx.workflow(cluster::workflows::cluster::Input {
			cluster_id: cluster_config.id,
			name_id: cluster_slug.to_string(),
			owner_team_id: None,
		})
		.tag("cluster_id", cluster_config.id)
		.dispatch()
		.await?;
		create_sub.next().await?;

		let cluster_res = ctx
			.op(cluster::ops::get::Input {
				cluster_ids: vec![cluster_config.id],
			})
			.await?;

		unwrap!(cluster_res.clusters.first().cloned())
	};

	// Get all datacenters
	let datacenter_list_res = ctx
		.op(cluster::ops::datacenter::list::Input {
			cluster_ids: vec![cluster_config.id],
		})
		.await?;
	let datacenter_ids = unwrap!(datacenter_list_res.clusters.first())
		.datacenter_ids
		.clone();
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::get::Input { datacenter_ids })
		.await?;

	// Log dcs that are trying to be deleted
	for existing_datacenter in &datacenters_res.datacenters {
		if !cluster_config
			.datacenters
			.contains_key(&existing_datacenter.name_id)
		{
			// Warn about removing datacenter
			tracing::warn!(
				dc_id = ?existing_datacenter.datacenter_id,
				dc_name = existing_datacenter.name_id,
				"deleting datacenters is currently unimplemented"
			);
		}
	}

	// Upsert datacenters
	for (dc_slug, dc_config) in &cluster_config.datacenters {
		let existing_datacenter = datacenters_res
			.datacenters
			.iter()
			.find(|x| x.datacenter_id == dc_config.id);
		upsert_datacenter(
			ctx,
			UpsertDatacenterArgs {
				use_autoscaler,
				cluster_id: cluster.cluster_id,
				dc_slug,
				dc_config,
				existing_datacenter,
			},
		)
		.await?;
	}

	Ok(())
}

struct UpsertDatacenterArgs<'a> {
	use_autoscaler: bool,
	cluster_id: Uuid,
	dc_slug: &'a str,
	dc_config: &'a config::rivet::Datacenter,
	existing_datacenter: Option<&'a cluster::types::Datacenter>,
}

/// Create or update an existing cluster.
async fn upsert_datacenter(
	ctx: &StandaloneCtx,
	UpsertDatacenterArgs {
		use_autoscaler,
		cluster_id,
		dc_slug,
		dc_config,
		existing_datacenter,
	}: UpsertDatacenterArgs<'_>,
) -> GlobalResult<()> {
	if let Some(existing_datacenter) = existing_datacenter {
		// Validate IDs match
		ensure_eq!(
			dc_slug,
			existing_datacenter.name_id,
			"datacenter id does not match config"
		);

		// Update existing datacenter
		let pools = dc_config
			.pools()
			.into_iter()
			.map(|(pool_type, pool)| {
				let desired_count = if use_autoscaler {
					None
				} else {
					Some(pool.desired_count)
				};

				cluster::types::PoolUpdate {
					pool_type: pool_type.into(),
					hardware: pool
						.hardware
						.into_iter()
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
			pools,
			prebakes_enabled: Some(dc_config.prebakes_enabled()),
		})
		.tag("datacenter_id", existing_datacenter.datacenter_id)
		.send()
		.await?;
	} else {
		// Create new datacenter

		let datacenter_id = dc_config.id;

		let (provider, provider_datacenter_id) = if let Some(provision) = &dc_config.provision {
			// Automatically provisioned
			(
				cluster::types::Provider::from(provision.provider),
				provision.provider_datacenter_id.clone(),
			)
		} else {
			// Manual node config
			(cluster::types::Provider::Manual, "dev".to_string())
		};

		let pools = dc_config
			.pools()
			.into_iter()
			.map(|(pool_type, pool)| cluster::types::Pool {
				pool_type: pool_type.into(),
				hardware: pool
					.hardware
					.into_iter()
					.map(Into::into)
					.collect::<Vec<_>>(),
				desired_count: pool.desired_count,
				min_count: pool.min_count,
				max_count: pool.max_count,
				drain_timeout: pool.drain_timeout,
			})
			.collect::<Vec<_>>();

		ctx.signal(cluster::workflows::cluster::DatacenterCreate {
			datacenter_id,
			name_id: dc_slug.to_string(),
			display_name: dc_config.name.clone(),

			provider,
			provider_datacenter_id,
			provider_api_token: None,

			pools,

			build_delivery_method: dc_config.build_delivery_method.into(),
			prebakes_enabled: dc_config.prebakes_enabled(),
		})
		.tag("cluster_id", cluster_id)
		.send()
		.await?;
	}

	Ok(())
}
