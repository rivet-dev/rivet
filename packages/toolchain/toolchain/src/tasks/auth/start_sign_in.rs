use anyhow::*;
use rivet_api::apis;
use serde::{Deserialize, Serialize};

use crate::{toolchain_ctx, util::task};

#[derive(Deserialize)]
pub struct Input {
	pub api_endpoint: String,
}

#[derive(Serialize)]
pub struct Output {
	pub device_link_url: String,
	pub device_link_token: String,
}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"auth.start_sign_in"
	}

	async fn run(_task: task::TaskCtx, input: Self::Input) -> Result<Self::Output> {
		let openapi_config_cloud_unauthed = apis::configuration::Configuration {
			base_path: input.api_endpoint,
			user_agent: Some(toolchain_ctx::user_agent()),
			..Default::default()
		};

		let prepare_res = apis::cloud_devices_links_api::cloud_devices_links_prepare(
			&openapi_config_cloud_unauthed,
		)
		.await?;

		Ok(Output {
			device_link_url: prepare_res.device_link_url,
			device_link_token: prepare_res.device_link_token,
		})
	}
}
