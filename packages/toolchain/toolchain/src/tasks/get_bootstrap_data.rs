use anyhow::*;
use rivet_api::apis;
use serde::{Deserialize, Serialize};

use crate::{project::environment::TEMPEnvironment, util::task};

#[derive(Deserialize)]
pub struct Input {}

#[derive(Serialize)]
pub struct Output {
	pub cloud: Option<CloudData>,
}

#[derive(Serialize)]
pub struct CloudData {
	pub token: String,
	pub api_endpoint: String,
	pub envs: Vec<TEMPEnvironment>,
}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"get_bootstrap_data"
	}

	async fn run(_task: task::TaskCtx, _input: Self::Input) -> Result<Self::Output> {
		let cloud = if let Some(ctx) = crate::toolchain_ctx::try_load().await? {
			// HACK: Map ns to temporary env data structure
			let envs = apis::cloud_games_api::cloud_games_get_game_by_id(
				&ctx.openapi_config_cloud,
				&ctx.project.game_id.to_string(),
				None,
			)
			.await?
			.game
			.namespaces
			.into_iter()
			.map(TEMPEnvironment::from)
			.collect::<Vec<_>>();

			Some(CloudData {
				token: ctx.access_token.clone(),
				api_endpoint: ctx.api_endpoint.clone(),
				envs,
			})
		} else {
			None
		};

		Ok(Output { cloud })
	}
}
