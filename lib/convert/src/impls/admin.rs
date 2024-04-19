use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiFrom;

impl ApiFrom<models::AdminPoolType> for backend::cluster::PoolType {
	fn api_from(value: models::AdminPoolType) -> backend::cluster::PoolType {
		match value {
			models::AdminPoolType::Job => backend::cluster::PoolType::Job,
			models::AdminPoolType::Gg => backend::cluster::PoolType::Gg,
			models::AdminPoolType::Ats => backend::cluster::PoolType::Ats,
		}
	}
}
