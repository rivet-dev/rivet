use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiTryFrom, ApiTryInto};

pub mod cdn;
pub mod identity;
pub mod kv;
pub mod matchmaker;

impl ApiTryFrom<backend::game::Version> for models::CloudVersionSummary {
	type Error = GlobalError;

	fn try_from(value: backend::game::Version) -> GlobalResult<Self> {
		Ok(models::CloudVersionSummary {
			version_id: internal_unwrap_owned!(value.version_id).as_uuid(),
			create_ts: util::timestamp::to_string(value.create_ts)?,
			display_name: value.display_name,
		})
	}
}
pub async fn config_to_proto(
	ctx: &OperationContext<()>,
	value: models::CloudVersionConfig,
) -> GlobalResult<backend::cloud::VersionConfig> {
	Ok(backend::cloud::VersionConfig {
		cdn: value.cdn.map(|x| (*x).try_into()).transpose()?,
		matchmaker: if let Some(matchmaker) = value.matchmaker {
			Some(matchmaker::config_to_proto(ctx, *matchmaker).await?)
		} else {
			None
		},
		kv: value.kv.map(|_| backend::kv::VersionConfig {}),
		identity: value.identity.map(|x| (*x).try_into()).transpose()?,
		// TODO:
		module: None,
	})
}

pub async fn config_to_openapi(
	ctx: &OperationContext<()>,
	value: backend::cloud::VersionConfig,
) -> GlobalResult<models::CloudVersionConfig> {
	Ok(models::CloudVersionConfig {
		engine: None, // CLient side only
		cdn: value
			.cdn
			.map(ApiTryFrom::try_from)
			.transpose()?
			.map(Box::new),
		matchmaker: if let Some(matchmaker) = value.matchmaker {
			Some(Box::new(
				matchmaker::config_to_openapi(ctx, matchmaker).await?,
			))
		} else {
			None
		},
		kv: value.kv.map(|_| serde_json::json!({})),
		identity: value
			.identity
			.map(ApiTryFrom::try_from)
			.transpose()?
			.map(Box::new),
	})
}
