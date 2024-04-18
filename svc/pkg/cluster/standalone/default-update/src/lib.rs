use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

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
	drain_timeout: u64,
}

#[derive(Deserialize)]
enum Provider {
	#[serde(rename = "linode")]
	Linode,
}

impl From<Provider> for backend::cluster::Provider {
	fn from(value: Provider) -> backend::cluster::Provider {
		match value {
			Provider::Linode => backend::cluster::Provider::Linode,
		}
	}
}

#[derive(Deserialize)]
struct Pool {
	hardware: Vec<Hardware>,
	desired_count: u32,
	max_count: u32,
}

#[derive(Deserialize, PartialEq, Eq, Hash)]
enum PoolType {
	#[serde(rename = "job")]
	Job,
	#[serde(rename = "gg")]
	Gg,
	#[serde(rename = "ats")]
	Ats,
}

impl From<PoolType> for backend::cluster::PoolType {
	fn from(value: PoolType) -> backend::cluster::PoolType {
		match value {
			PoolType::Job => backend::cluster::PoolType::Job,
			PoolType::Gg => backend::cluster::PoolType::Gg,
			PoolType::Ats => backend::cluster::PoolType::Ats,
		}
	}
}

#[derive(Deserialize)]
struct Hardware {
	name: String,
}

impl From<Hardware> for backend::cluster::Hardware {
	fn from(value: Hardware) -> backend::cluster::Hardware {
		backend::cluster::Hardware {
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

impl From<BuildDeliveryMethod> for backend::cluster::BuildDeliveryMethod {
	fn from(value: BuildDeliveryMethod) -> backend::cluster::BuildDeliveryMethod {
		match value {
			BuildDeliveryMethod::TrafficServer => {
				backend::cluster::BuildDeliveryMethod::TrafficServer
			}
			BuildDeliveryMethod::S3Direct => backend::cluster::BuildDeliveryMethod::S3Direct,
		}
	}
}

#[tracing::instrument]
pub async fn run_from_env(use_autoscaler: bool) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("cluster-default-update").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-default-update");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"cluster-default-update".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	// Read config from env
	let Some(config_json) = util::env::var("RIVET_DEFAULT_CLUSTER_CONFIG").ok() else {
		tracing::warn!("no cluster config set in namespace config");
		return Ok(());
	};
	let config = serde_json::from_str::<Cluster>(&config_json)?;

	let taint = util::env::var("RIVET_TAINT_DEFAULT_CLUSTER")
		.ok()
		.unwrap_or_else(|| "0".to_string())
		== "1";

	// HACK: When deploying both monolith worker and this service for the first time, there is a race
	// condition which might result in the message being published from here but not caught by
	// monolith-worker, resulting in nothing happening.
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	let cluster_id = util_cluster::default_cluster_id();

	let (cluster_res, datacenter_list_res) = tokio::try_join!(
		// Check if cluster already exists
		op!([ctx] cluster_get {
			cluster_ids: vec![cluster_id.into()],
		}),
		op!([ctx] cluster_datacenter_list {
			cluster_ids: vec![cluster_id.into()],
		}),
	)?;

	// Get all datacenters
	let cluster = unwrap!(datacenter_list_res.clusters.first());
	let datacenters_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: cluster.datacenter_ids.clone(),
	})
	.await?;

	if cluster_res.clusters.is_empty() {
		tracing::warn!("creating default cluster");

		msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
			cluster_id: Some(cluster_id.into()),
			name_id: config.name_id.clone(),
			owner_team_id: None,
		})
		.await?;
	}

	for existing_datacenter in &datacenters_res.datacenters {
		let datacenter_id = unwrap_ref!(existing_datacenter.datacenter_id).as_uuid();

		if !config
			.datacenters
			.iter()
			.any(|(_, dc)| dc.datacenter_id == datacenter_id)
		{
			// TODO: Delete datacenters
		}
	}

	for (name_id, datacenter) in config.datacenters {
		let datacenter_id_proto = Some(datacenter.datacenter_id.into());
		let existing_datacenter = datacenters_res
			.datacenters
			.iter()
			.any(|dc| dc.datacenter_id == datacenter_id_proto);

		// Update existing datacenter
		if existing_datacenter {
			let new_pools = datacenter
				.pools
				.into_iter()
				.map(|(pool_type, pool)| {
					let desired_count = match pool_type {
						PoolType::Ats => Some(pool.desired_count),
						PoolType::Job | PoolType::Gg => {
							if use_autoscaler {
								None
							} else {
								Some(pool.desired_count)
							}
						}
					};

					cluster::msg::datacenter_update::PoolUpdate {
						pool_type: Into::<backend::cluster::PoolType>::into(pool_type) as i32,
						hardware: pool
							.hardware
							.into_iter()
							.map(Into::into)
							.collect::<Vec<_>>(),
						desired_count,
						max_count: Some(pool.max_count),
					}
				})
				.collect::<Vec<_>>();

			msg!([ctx] @wait cluster::msg::datacenter_update(datacenter.datacenter_id) {
				datacenter_id: datacenter_id_proto,
				pools: new_pools,
				// Convert from seconds to ms
				drain_timeout: Some(datacenter.drain_timeout * 1000),
			})
			.await?;
		}
		// Create new datacenter
		else {
			msg!([ctx] @wait cluster::msg::datacenter_create(datacenter.datacenter_id) {
				datacenter_id: datacenter_id_proto,
				cluster_id: Some(cluster_id.into()),
				name_id,
				display_name: datacenter.display_name,

				provider: Into::<backend::cluster::Provider>::into(datacenter.provider) as i32,
				provider_datacenter_id: datacenter.provider_datacenter_name,
				provider_api_token: None,

				pools: datacenter.pools.into_iter().map(|(pool_type, pool)| {
					backend::cluster::Pool {
						pool_type: Into::<backend::cluster::PoolType>::into(pool_type) as i32,
						hardware: pool.hardware.into_iter().map(Into::into).collect::<Vec<_>>(),
						desired_count: pool.desired_count,
						max_count: pool.max_count,
					}
				}).collect::<Vec<_>>(),

				build_delivery_method: Into::<backend::cluster::BuildDeliveryMethod>::into(datacenter.build_delivery_method) as i32,
				drain_timeout: datacenter.drain_timeout,
			})
			.await?;
		}

		// TODO: Both this message and datacenter-create/datacenter-update (above) publish datacenter-scale.
		// This results in double provisioning until datacenter-scale is published again, cleaning up the
		// excess.
		// Taint datacenter
		if taint {
			msg!([ctx] @wait cluster::msg::datacenter_taint(datacenter.datacenter_id) {
				datacenter_id: datacenter_id_proto,
			})
			.await?;
		}
	}

	Ok(())
}
