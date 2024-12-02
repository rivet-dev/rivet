use anyhow::*;
use rivet_api::apis;
use serde::{Deserialize, Serialize};

use crate::{meta, paths, toolchain_ctx, util::task};

#[derive(Deserialize)]
pub struct Input {
	pub api_endpoint: String,
	pub device_link_token: String,
}

#[derive(Serialize)]
pub struct Output {}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"auth.wait_for_sign_in"
	}

	async fn run(_task: task::TaskCtx, input: Self::Input) -> Result<Self::Output> {
		let openapi_config_cloud_unauthed = apis::configuration::Configuration {
			base_path: input.api_endpoint.clone(),
			user_agent: Some(toolchain_ctx::user_agent()),
			..Default::default()
		};

		let mut watch_index = None;
		let token = loop {
			let prepare_res = apis::cloud_devices_links_api::cloud_devices_links_get(
				&openapi_config_cloud_unauthed,
				&input.device_link_token,
				watch_index.as_ref().map(String::as_str),
			)
			.await?;

			watch_index = Some(prepare_res.watch.index);

			if let Some(token) = prepare_res.cloud_token {
				break token;
			}
		};

		let new_ctx = crate::toolchain_ctx::init(input.api_endpoint.clone(), token.clone()).await?;

		let inspect_res =
			apis::cloud_auth_api::cloud_auth_inspect(&new_ctx.openapi_config_cloud).await?;

		let game_id = inspect_res
			.agent
			.game_cloud
			.context("no game cloud token found")?
			.game_id;

		let _game_res = apis::cloud_games_api::cloud_games_get_game_by_id(
			&new_ctx.openapi_config_cloud,
			&game_id.to_string(),
			None,
		)
		.await?;

		meta::mutate_project(&paths::data_dir()?, |meta| {
			meta.cloud = Some(meta::Cloud::new(input.api_endpoint, token))
		})
		.await?;

		Ok(Output {})
	}
}
