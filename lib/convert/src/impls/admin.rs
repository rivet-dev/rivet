use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom};

impl ApiFrom<models::AdminClustersPoolType> for backend::cluster::PoolType {
	fn api_from(value: models::AdminClustersPoolType) -> backend::cluster::PoolType {
		match value {
			models::AdminClustersPoolType::Job => backend::cluster::PoolType::Job,
			models::AdminClustersPoolType::Gg => backend::cluster::PoolType::Gg,
			models::AdminClustersPoolType::Ats => backend::cluster::PoolType::Ats,
		}
	}
}

impl ApiFrom<backend::cluster::PoolType> for models::AdminClustersPoolType {
	fn api_from(value: backend::cluster::PoolType) -> models::AdminClustersPoolType {
		match value {
			backend::cluster::PoolType::Job => models::AdminClustersPoolType::Job,
			backend::cluster::PoolType::Gg => models::AdminClustersPoolType::Gg,
			backend::cluster::PoolType::Ats => models::AdminClustersPoolType::Ats,
		}
	}
}

impl ApiFrom<models::AdminClustersProvider> for backend::cluster::Provider {
	fn api_from(value: models::AdminClustersProvider) -> backend::cluster::Provider {
		match value {
			models::AdminClustersProvider::Linode => backend::cluster::Provider::Linode,
		}
	}
}

impl ApiFrom<backend::cluster::Provider> for models::AdminClustersProvider {
	fn api_from(value: backend::cluster::Provider) -> models::AdminClustersProvider {
		match value {
			backend::cluster::Provider::Linode => models::AdminClustersProvider::Linode,
		}
	}
}

impl ApiFrom<models::AdminClustersBuildDeliveryMethod> for backend::cluster::BuildDeliveryMethod {
	fn api_from(value: models::AdminClustersBuildDeliveryMethod) -> backend::cluster::BuildDeliveryMethod {
		match value {
			models::AdminClustersBuildDeliveryMethod::TrafficServer => {
				backend::cluster::BuildDeliveryMethod::TrafficServer
			}
			models::AdminClustersBuildDeliveryMethod::S3Direct => {
				backend::cluster::BuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiFrom<backend::cluster::BuildDeliveryMethod> for models::AdminClustersBuildDeliveryMethod {
	fn api_from(value: backend::cluster::BuildDeliveryMethod) -> models::AdminClustersBuildDeliveryMethod {
		match value {
			backend::cluster::BuildDeliveryMethod::TrafficServer => {
				models::AdminClustersBuildDeliveryMethod::TrafficServer
			}
			backend::cluster::BuildDeliveryMethod::S3Direct => {
				models::AdminClustersBuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiTryFrom<backend::cluster::Cluster> for models::AdminClustersCluster {
	type Error = GlobalError;

	fn api_try_from(value: backend::cluster::Cluster) -> GlobalResult<models::AdminClustersCluster> {
		Ok(models::AdminClustersCluster {
			cluster_id: unwrap!(value.cluster_id).into(),
			name_id: value.name_id,
			create_ts: value.create_ts,
			owner_team_id: value.owner_team_id.map(Into::into),
		})
	}
}

impl ApiTryFrom<backend::cluster::Datacenter> for models::AdminClustersDatacenter {
	type Error = GlobalError;

	fn api_try_from(value: backend::cluster::Datacenter) -> GlobalResult<models::AdminClustersDatacenter> {
		Ok(models::AdminClustersDatacenter {
			build_delivery_method: unwrap!(backend::cluster::BuildDeliveryMethod::from_i32(
				value.build_delivery_method
			))
			.api_into(),
			cluster_id: unwrap!(value.cluster_id).into(),
			datacenter_id: unwrap!(value.datacenter_id).into(),
			display_name: value.display_name,
			name_id: value.name_id,
			pools: value
				.pools
				.iter()
				.map(|p| {
					Ok(models::AdminClustersPool {
						desired_count: unwrap!(p.desired_count.try_into()),
						drain_timeout: unwrap!(p.drain_timeout.try_into()),
						hardware: p
							.hardware
							.iter()
							.map(|h| {
								Ok(models::AdminClustersHardware {
									provider_hardware: h.provider_hardware.clone(),
								})
							})
							.collect::<Result<Vec<models::AdminClustersHardware>, GlobalError>>()?,
						min_count: unwrap!(p.min_count.try_into()),
						max_count: unwrap!(p.max_count.try_into()),
						pool_type: unwrap!(backend::cluster::PoolType::from_i32(p.pool_type))
							.api_into(),
					})
				})
				.collect::<Result<Vec<_>, GlobalError>>()?,
			provider: unwrap!(backend::cluster::Provider::from_i32(value.provider)).api_into(),
			provider_api_token: value.provider_api_token,
			provider_datacenter_id: value.provider_datacenter_id,
		})
	}
}
