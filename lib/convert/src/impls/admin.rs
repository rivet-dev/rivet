use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom};

impl ApiFrom<models::AdminPoolType> for backend::cluster::PoolType {
	fn api_from(value: models::AdminPoolType) -> backend::cluster::PoolType {
		match value {
			models::AdminPoolType::Job => backend::cluster::PoolType::Job,
			models::AdminPoolType::Gg => backend::cluster::PoolType::Gg,
			models::AdminPoolType::Ats => backend::cluster::PoolType::Ats,
		}
	}
}

impl ApiFrom<backend::cluster::PoolType> for models::AdminPoolType {
	fn api_from(value: backend::cluster::PoolType) -> models::AdminPoolType {
		match value {
			backend::cluster::PoolType::Job => models::AdminPoolType::Job,
			backend::cluster::PoolType::Gg => models::AdminPoolType::Gg,
			backend::cluster::PoolType::Ats => models::AdminPoolType::Ats,
		}
	}
}

impl ApiFrom<models::AdminProvider> for backend::cluster::Provider {
	fn api_from(value: models::AdminProvider) -> backend::cluster::Provider {
		match value {
			models::AdminProvider::Linode => backend::cluster::Provider::Linode,
		}
	}
}

impl ApiFrom<backend::cluster::Provider> for models::AdminProvider {
	fn api_from(value: backend::cluster::Provider) -> models::AdminProvider {
		match value {
			backend::cluster::Provider::Linode => models::AdminProvider::Linode,
		}
	}
}

impl ApiFrom<models::AdminBuildDeliveryMethod> for backend::cluster::BuildDeliveryMethod {
	fn api_from(value: models::AdminBuildDeliveryMethod) -> backend::cluster::BuildDeliveryMethod {
		match value {
			models::AdminBuildDeliveryMethod::TrafficServer => {
				backend::cluster::BuildDeliveryMethod::TrafficServer
			}
			models::AdminBuildDeliveryMethod::S3Direct => {
				backend::cluster::BuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiFrom<backend::cluster::BuildDeliveryMethod> for models::AdminBuildDeliveryMethod {
	fn api_from(value: backend::cluster::BuildDeliveryMethod) -> models::AdminBuildDeliveryMethod {
		match value {
			backend::cluster::BuildDeliveryMethod::TrafficServer => {
				models::AdminBuildDeliveryMethod::TrafficServer
			}
			backend::cluster::BuildDeliveryMethod::S3Direct => {
				models::AdminBuildDeliveryMethod::S3Direct
			}
		}
	}
}

impl ApiTryFrom<backend::cluster::Cluster> for models::AdminCluster {
	type Error = GlobalError;

	fn api_try_from(value: backend::cluster::Cluster) -> GlobalResult<models::AdminCluster> {
		Ok(models::AdminCluster {
			cluster_id: unwrap!(value.cluster_id).into(),
			name_id: value.name_id,
			create_ts: value.create_ts,
			owner_team_id: value.owner_team_id.map(Into::into),
		})
	}
}

impl ApiTryFrom<backend::cluster::Datacenter> for models::AdminDatacenter {
	type Error = GlobalError;

	fn api_try_from(value: backend::cluster::Datacenter) -> GlobalResult<models::AdminDatacenter> {
		Ok(models::AdminDatacenter {
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
					Ok(models::AdminPool {
						desired_count: unwrap!(p.desired_count.try_into()),
						drain_timeout: unwrap!(p.drain_timeout.try_into()),
						hardware: p
							.hardware
							.iter()
							.map(|h| {
								Ok(models::AdminHardware {
									provider_hardware: h.provider_hardware.clone(),
								})
							})
							.collect::<Result<Vec<models::AdminHardware>, GlobalError>>()?,
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
