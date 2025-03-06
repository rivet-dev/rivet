use anyhow::*;
use pkg_version::{pkg_version_major, pkg_version_minor, pkg_version_patch};
use rivet_api::{apis, models};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::{meta, paths};

pub const VERSION: &str = {
	const MAJOR: u32 = pkg_version_major!();
	const MINOR: u32 = pkg_version_minor!();
	const PATCH: u32 = pkg_version_patch!();
	const_format::formatcp!("{MAJOR}.{MINOR}.{PATCH}")
};

pub fn user_agent() -> String {
	format!("CLI/{VERSION}")
}

pub type ToolchainCtx = Arc<CtxInner>;

pub struct CtxInner {
	pub api_endpoint: String,
	pub access_token: String,
	pub project: models::CloudGameFull,

	/// Domains that host parts of Rivet
	pub bootstrap: rivet_api::models::CloudBootstrapResponse,

	pub openapi_config_cloud: apis::configuration::Configuration,
	pub openapi_config_actor: apis::configuration::Configuration,
}

static TOOLCHAIN_CTX: OnceCell<ToolchainCtx> = OnceCell::const_new();

pub async fn try_load() -> Result<Option<ToolchainCtx>> {
	let data = meta::read_project(&paths::data_dir()?, |x| {
		x.cloud
			.as_ref()
			.map(|cloud| (cloud.api_endpoint.clone(), cloud.cloud_token.clone()))
	})
	.await?;
	if let Some((api_endpoint, token)) = data {
		let ctx = TOOLCHAIN_CTX
			.get_or_try_init(|| async { init(api_endpoint, token).await })
			.await?;
		Ok(Some(ctx.clone()))
	} else {
		Ok(None)
	}
}

pub async fn load() -> Result<ToolchainCtx> {
	let (api_endpoint, token) = meta::try_read_project(&paths::data_dir()?, |x| {
		let cloud = x.cloud()?;
		Ok((cloud.api_endpoint.clone(), cloud.cloud_token.clone()))
	})
	.await?;
	let ctx = TOOLCHAIN_CTX
		.get_or_try_init(|| async { init(api_endpoint, token).await })
		.await?;
	Ok(ctx.clone())
}

pub async fn init(api_endpoint: String, cloud_token: String) -> Result<ToolchainCtx> {
	// Disable connection pooling to fix "connection closed before message completed"
	//
	// See https://github.com/hyperium/hyper/issues/2136#issuecomment-861826148
	let client = reqwest::Client::builder()
		.no_proxy()
		.pool_max_idle_per_host(0)
		.build()?;

	// Create OpenAPI config
	let openapi_config_cloud = apis::configuration::Configuration {
		base_path: api_endpoint.clone(),
		bearer_access_token: Some(cloud_token.clone()),
		user_agent: Some(user_agent()),
		client: client.clone(),
		..Default::default()
	};

	// Create OpenAPI config
	let openapi_config_actor = apis::configuration::Configuration {
		// TODO: Don't hardcode
		base_path: "http://localhost:8083".to_string(),
		bearer_access_token: Some(cloud_token.clone()),
		user_agent: Some(user_agent()),
		client,
		..Default::default()
	};

	// Make requests
	let (inspect_response, bootstrap_response): (
		rivet_api::models::CloudInspectResponse,
		rivet_api::models::CloudBootstrapResponse,
	) = tokio::try_join!(
		async {
			Result::Ok(
				apis::cloud_auth_api::cloud_auth_inspect(&openapi_config_cloud)
					.await
					.context("inspect failed")?,
			)
		},
		async {
			Result::Ok(
				apis::cloud_api::cloud_bootstrap(&openapi_config_cloud)
					.await
					.context("bootstrap failed")?,
			)
		}
	)?;

	let project_id = if let Some(game_cloud) = inspect_response.agent.game_cloud {
		game_cloud.game_id
	} else {
		bail!("invalid agent kind")
	};

	let project_res = apis::cloud_games_api::cloud_games_get_game_by_id(
		&openapi_config_cloud,
		&project_id.to_string(),
		None,
	)
	.await
	.context("get project failed")?;

	Ok(Arc::new(CtxInner {
		api_endpoint,
		access_token: cloud_token,
		project: *project_res.game,
		bootstrap: bootstrap_response,
		openapi_config_cloud,
		openapi_config_actor,
	}))
}
