use anyhow::*;
use rivet_api::{apis, models};
use serde::Serialize;
use uuid::Uuid;

use crate::ToolchainCtx;

// TODO: Replace this with a production API
#[derive(Clone, Debug, Serialize)]
pub struct TEMPEnvironment {
	pub id: Uuid,
	pub created_at: String,
	pub slug: String,
	pub name: String,
}

impl From<models::CloudNamespaceSummary> for TEMPEnvironment {
	fn from(ns: models::CloudNamespaceSummary) -> Self {
		TEMPEnvironment {
			id: ns.namespace_id,
			created_at: ns.create_ts,
			slug: ns.name_id,
			name: ns.display_name,
		}
	}
}

impl From<models::CloudNamespaceFull> for TEMPEnvironment {
	fn from(ns: models::CloudNamespaceFull) -> Self {
		TEMPEnvironment {
			id: ns.namespace_id,
			created_at: ns.create_ts,
			slug: ns.name_id,
			name: ns.display_name,
		}
	}
}

pub async fn get_env(ctx: &ToolchainCtx, env_id: Uuid) -> Result<TEMPEnvironment> {
	let res = apis::cloud_games_namespaces_api::cloud_games_namespaces_get_game_namespace_by_id(
		&ctx.openapi_config_cloud,
		&ctx.project.game_id.to_string(),
		&env_id.to_string(),
	)
	.await?;

	Ok(TEMPEnvironment::from(*res.namespace))
}
