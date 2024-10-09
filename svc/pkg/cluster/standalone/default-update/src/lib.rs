use std::collections::HashMap;

use chirp_workflow::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Cluster {
	name_id: String,
	datacenters: HashMap<String, Datacenter>,
}

#[derive(Deserialize)]
struct Datacenter {
	datacenter_id: Uuid,
	display_name: String,
	provider: Provider,
	provider_datacenter_name: String,
	pools: HashMap<PoolType, Pool>,
	build_delivery_method: BuildDeliveryMethod,
	prebakes_enabled: bool,
}

#[derive(Deserialize)]
enum Provider {
	#[serde(rename = "linode")]
	Linode,
}

impl From<Provider> for cluster::types::Provider {
	fn from(value: Provider) -> cluster::types::Provider {
		match value {
			Provider::Linode => cluster::types::Provider::Linode,
		}
	}
}

#[derive(Deserialize)]
struct Pool {
	hardware: Vec<Hardware>,
	desired_count: u32,
	min_count: u32,
	max_count: u32,
	drain_timeout: u64,
}

#[derive(Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum PoolType {
	Job,
	Gg,
	Ats,
	Pegboard,
}

impl From<PoolType> for cluster::types::PoolType {
	fn from(value: PoolType) -> cluster::types::PoolType {
		match value {
			PoolType::Job => cluster::types::PoolType::Job,
			PoolType::Gg => cluster::types::PoolType::Gg,
			PoolType::Ats => cluster::types::PoolType::Ats,
			PoolType::Pegboard => cluster::types::PoolType::Pegboard,
		}
	}
}

#[derive(Deserialize)]
struct Hardware {
	name: String,
}

impl From<Hardware> for cluster::types::Hardware {
	fn from(value: Hardware) -> cluster::types::Hardware {
		cluster::types::Hardware {
			provider_hardware: value.name,
		}
	}
}

#[derive(Deserialize)]
enum BuildDeliveryMethod {
	#[serde(rename = "traffic_server")]
	TrafficServer,
	#[serde(rename = "s3_direct")]
	S3Direct,
}

impl From<BuildDeliveryMethod> for cluster::types::BuildDeliveryMethod {
	fn from(value: BuildDeliveryMethod) -> cluster::types::BuildDeliveryMethod {
		match value {
			BuildDeliveryMethod::TrafficServer => {
				cluster::types::BuildDeliveryMethod::TrafficServer
			}
			BuildDeliveryMethod::S3Direct => cluster::types::BuildDeliveryMethod::S3Direct,
		}
	}
}

#[tracing::instrument]
pub async fn run_from_env(use_autoscaler: bool) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("cluster-default-update").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-default-update");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-default-update",
	)
	.await?;

	// Read config from env
	let Some(config_json) = util::env::var("RIVET_DEFAULT_CLUSTER_CONFIG").ok() else {
		tracing::warn!("no cluster config set in namespace config");
		return Ok(());
	};
	let config = serde_json::from_str::<Cluster>(&config_json)?;

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
	let datacenters_res = ctx.op(cluster::ops::datacenter::get::Input {
		datacenter_ids: cluster.datacenter_ids.clone(),
	}).await?;

	if cluster_res.clusters.is_empty() {
		tracing::warn!("creating default cluster");

		ctx.workflow(cluster::workflows::cluster::Input {
			cluster_id,
			name_id: config.name_id.clone(),
			owner_team_id: None,
		}).tag("cluster_id", cluster_id,).dispatch().await?;
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

	for (name_id, datacenter) in config.datacenters {
		let existing_datacenter = datacenters_res
			.datacenters
			.iter()
			.any(|dc| dc.datacenter_id == datacenter.datacenter_id);

		// Update existing datacenter
		if existing_datacenter {
			let new_pools = datacenter
				.pools
				.into_iter()
				.map(|(pool_type, pool)| {
					let desired_count = match pool_type {
						PoolType::Ats | PoolType::Gg => Some(pool.desired_count),
						PoolType::Job | PoolType::Pegboard => {
							if use_autoscaler {
								None
							} else {
								Some(pool.desired_count)
							}
						}
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
				pools: new_pools,
				prebakes_enabled: Some(datacenter.prebakes_enabled),
			}).tag("datacenter_id", datacenter.datacenter_id,).send().await?;
		}
		// Create new datacenter
		else {
			ctx.signal(cluster::workflows::cluster::DatacenterCreate {
				datacenter_id: datacenter.datacenter_id,
				name_id,
				display_name: datacenter.display_name,
	
				provider: datacenter.provider.into(),
				provider_datacenter_id: datacenter.provider_datacenter_name,
				provider_api_token: None,
	
				pools: datacenter.pools.into_iter().map(|(pool_type, pool)| {
					cluster::types::Pool {
						pool_type: pool_type.into(),
						hardware: pool.hardware.into_iter().map(Into::into).collect::<Vec<_>>(),
						desired_count: pool.desired_count,
						min_count: pool.min_count,
						max_count: pool.max_count,
						drain_timeout: pool.drain_timeout,
					}
				}).collect::<Vec<_>>(),
	
				build_delivery_method: datacenter.build_delivery_method.into(),
				prebakes_enabled: datacenter.prebakes_enabled,
			}).tag("cluster_id", cluster_id,).send().await?;
		}
	}

	Ok(())
}
