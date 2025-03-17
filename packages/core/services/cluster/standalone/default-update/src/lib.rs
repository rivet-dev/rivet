use chirp_workflow::prelude::*;
use rivet_config::config;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	// TODO: When running bolt up, this service gets created first before `cluster-worker` so the messages
	// sent from here are received but effectively forgotten because `cluster-worker` gets restarted
	// immediately afterwards. This server will be replaced with a bolt infra step
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;

	start_inner(config, pools).await
}

#[tracing::instrument(skip_all)]
pub async fn start_inner(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-default-update");
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseCrdbNats::from_pools(pools.clone())?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-default-update",
	)
	.await?;

	let rivet_config = &ctx.config().server()?.rivet;

	let cluster_configs = rivet_config.clusters();

	for (cluster_slug, cluster) in cluster_configs.iter() {
		upsert_cluster(&ctx, cluster_slug, cluster).await?;
	}

	Ok(())
}

/// Creates or updates an existing cluster.
async fn upsert_cluster(
	ctx: &StandaloneCtx,
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
			.bootstrap_datacenters
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
	for (dc_slug, dc_config) in &cluster_config.bootstrap_datacenters {
		let existing_datacenter = datacenters_res
			.datacenters
			.iter()
			.find(|x| x.datacenter_id == dc_config.id);
		upsert_datacenter(
			ctx,
			UpsertDatacenterArgs {
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
	cluster_id: Uuid,
	dc_slug: &'a str,
	dc_config: &'a config::rivet::Datacenter,
	existing_datacenter: Option<&'a cluster::types::Datacenter>,
}

/// Create or update an existing cluster.
async fn upsert_datacenter(
	ctx: &StandaloneCtx,
	UpsertDatacenterArgs {
		cluster_id,
		dc_slug,
		dc_config,
		existing_datacenter,
	}: UpsertDatacenterArgs<'_>,
) -> GlobalResult<()> {
	let guard_public_hostname = match &dc_config.guard.public_hostname {
		Some(rivet_config::config::server::rivet::GuardPublicHostname::DnsParent(x)) => {
			Some(cluster::types::GuardPublicHostname::DnsParent(x.clone()))
		}
		Some(rivet_config::config::server::rivet::GuardPublicHostname::Static(x)) => {
			Some(cluster::types::GuardPublicHostname::Static(x.clone()))
		}
		None => None,
	};

	if let Some(existing_datacenter) = existing_datacenter {
		// Validate IDs match
		ensure_eq!(
			dc_slug,
			existing_datacenter.name_id,
			"datacenter id does not match config"
		);

		ctx.signal(cluster::workflows::datacenter::Update {
			pools: Vec::new(),
			prebakes_enabled: None,
			guard_public_hostname,
		})
		.tag("datacenter_id", existing_datacenter.datacenter_id)
		.send()
		.await?;
	} else {
		// Create new datacenter

		let datacenter_id = dc_config.id;

		let provider = cluster::types::Provider::Manual;
		let provider_datacenter_id = "dev".to_string();

		ctx.signal(cluster::workflows::cluster::DatacenterCreate {
			datacenter_id,
			name_id: dc_slug.to_string(),
			display_name: dc_config.name.clone(),

			provider,
			provider_datacenter_id,
			provider_api_token: None,

			pools: Vec::new(),

			build_delivery_method: dc_config.build_delivery_method.into(),
			prebakes_enabled: false,
			guard_public_hostname,
		})
		.tag("cluster_id", cluster_id)
		.send()
		.await?;
	}

	Ok(())
}
