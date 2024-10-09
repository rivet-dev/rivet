// NOTE: This file should be moved to cluster/types.rs but there is a circular dependency with region-get

use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom};

impl ApiFrom<models::AdminClustersPoolType> for cluster::types::PoolType {
	fn api_from(value: models::AdminClustersPoolType) -> cluster::types::PoolType {
		match value {
			models::AdminClustersPoolType::Job => cluster::types::PoolType::Job,
			models::AdminClustersPoolType::Gg => cluster::types::PoolType::Gg,
			models::AdminClustersPoolType::Ats => cluster::types::PoolType::Ats,
		}
	}
}

impl ApiFrom<cluster::types::PoolType> for models::AdminClustersPoolType {
	fn api_from(value: cluster::types::PoolType) -> models::AdminClustersPoolType {
		match value {
			cluster::types::PoolType::Job => models::AdminClustersPoolType::Job,
			cluster::types::PoolType::Gg => models::AdminClustersPoolType::Gg,
			cluster::types::PoolType::Ats => models::AdminClustersPoolType::Ats,
		}
	}
}

impl ApiFrom<models::AdminClustersProvider> for cluster::types::Provider {
	fn api_from(value: models::AdminClustersProvider) -> cluster::types::Provider {
		match value {
			models::AdminClustersProvider::Linode => cluster::types::Provider::Linode,
		}
	}
}

impl ApiFrom<cluster::types::Provider> for models::AdminClustersProvider {
	fn api_from(value: cluster::types::Provider) -> models::AdminClustersProvider {
		match value {
			cluster::types::Provider::Linode => models::AdminClustersProvider::Linode,
		}
	}
}

impl ApiFrom<models::AdminClustersBuildDeliveryMethod> for cluster::types::BuildDeliveryMethod {
	fn api_from(value: models::AdminClustersBuildDeliveryMethod) -> cluster::types::BuildDeliveryMethod {
		match value {
			models::AdminClustersBuildDeliveryMethod::TrafficServer => {
				cluster::types::BuildDeliveryMethod::TrafficServer
			}
			models::AdminClustersBuildDeliveryMethod::S3Direct => {
				cluster::types::BuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiFrom<cluster::types::BuildDeliveryMethod> for models::AdminClustersBuildDeliveryMethod {
	fn api_from(value: cluster::types::BuildDeliveryMethod) -> models::AdminClustersBuildDeliveryMethod {
		match value {
			cluster::types::BuildDeliveryMethod::TrafficServer => {
				models::AdminClustersBuildDeliveryMethod::TrafficServer
			}
			cluster::types::BuildDeliveryMethod::S3Direct => {
				models::AdminClustersBuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiTryFrom<cluster::types::Cluster> for models::AdminClustersCluster {
	type Error = GlobalError;

	fn api_try_from(value: cluster::types::Cluster) -> GlobalResult<models::AdminClustersCluster> {
		Ok(models::AdminClustersCluster {
			cluster_id: value.cluster_id,
			name_id: value.name_id,
			create_ts: value.create_ts,
			owner_team_id: value.owner_team_id.map(Into::into),
		})
	}
}

impl ApiTryFrom<cluster::types::Datacenter> for models::AdminClustersDatacenter {
	type Error = GlobalError;

	fn api_try_from(value: cluster::types::Datacenter) -> GlobalResult<models::AdminClustersDatacenter> {
		Ok(models::AdminClustersDatacenter {
			build_delivery_method: value.build_delivery_method.api_into(),
			cluster_id: value.cluster_id,
			datacenter_id: value.datacenter_id,
			display_name: value.display_name,
			name_id: value.name_id,
			pools: value
				.pools
				.iter()
				.map(|p| {
					Ok(models::AdminClustersPool {
						desired_count: p.desired_count.try_into()?,
						drain_timeout_ms: p.drain_timeout.try_into()?,
						hardware: p
							.hardware
							.iter()
							.map(|h| {
								models::AdminClustersHardware {
									provider_hardware: h.provider_hardware.clone(),
								}
							})
							.collect::<Vec<_>>(),
						min_count: p.min_count.try_into()?,
						max_count: p.max_count.try_into()?,
						pool_type: p.pool_type.api_into(),
					})
				})
				.collect::<GlobalResult<Vec<_>>>()?,
			provider: value.provider.api_into(),
			provider_datacenter_id: value.provider_datacenter_id,
			prebakes_enabled: value.prebakes_enabled,
		})
	}
}

impl ApiFrom<models::AdminClustersPoolUpdate> for cluster::types::PoolUpdate {
	fn api_from(value: models::AdminClustersPoolUpdate) -> cluster::types::PoolUpdate {
		cluster::types::PoolUpdate {
			pool_type: value.pool_type.api_into(),
			hardware: value
				.hardware
				.iter()
				.map(|h| cluster::types::Hardware {
					provider_hardware: h.provider_hardware.clone(),
				})
				.collect(),
			desired_count: value.desired_count.map(|c| c as u32),
			min_count: value.min_count.map(|c| c as u32),
			max_count: value.max_count.map(|c| c as u32),
			drain_timeout: value.drain_timeout.map(|d| d as u64),
		}
	}
}

impl ApiTryFrom<cluster::types::Server> for models::AdminClustersServer {
	type Error = GlobalError;

	fn api_try_from(value: cluster::types::Server) -> GlobalResult<models::AdminClustersServer> {
		Ok(models::AdminClustersServer {
			server_id: value.server_id,
			datacenter_id: value.datacenter_id,
			pool_type: value.pool_type.api_into(),
			public_ip: value.public_ip.map(|ip| ip.to_string()),
		})
	}
}