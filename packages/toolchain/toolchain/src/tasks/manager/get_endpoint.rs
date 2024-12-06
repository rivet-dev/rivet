use anyhow::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rivet_api::{apis, models};

use crate::{meta, paths, util::task};

#[derive(Deserialize)]
pub struct Input {
	pub env_slug: String,
}

#[derive(Serialize)]
pub struct Output {
	pub endpoint: String,
}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"manager_get_endpoint"
	}

	async fn run(task: task::TaskCtx, input: Self::Input) -> Result<Self::Output> {
		let ctx = crate::toolchain_ctx::load().await?;

		// Check if manager exists
		let res = apis::actor_api::actor_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id),
			Some(&input.env_slug),
			Some(&serde_json::to_string(&serde_json::json!({
				"name": "manager",
			}))?),
			Some(false),
			None,
		)
		.await?;
		if res.actors.len() > 1 {
			task.log("WARNING: More than 1 manager actor is running. We recommend manually stopping one of them.")
		}
		let Some(actor) = res.actors.into_iter().next() else {
			bail!("manager actor does not exist")
		};

		// Get endpoint
		let http_port = actor
			.network
			.ports
			.get(crate::util::actor_manager::HTTP_PORT)
			.context("missing http port")?;
		let protocol = match http_port.protocol {
			models::ActorPortProtocol::Http | models::ActorPortProtocol::Tcp => "http",
			models::ActorPortProtocol::Https => "https",
			models::ActorPortProtocol::TcpTls | models::ActorPortProtocol::Udp => {
				bail!("unsupported protocol")
			}
		};
		let public_hostname = http_port
			.public_hostname
			.as_ref()
			.context("missing public_hostname")?;
		let public_port = http_port
			.public_port
			.as_ref()
			.context("missing public_port")?;
		let endpoint = format!("{protocol}://{public_hostname}:{public_port}");

		Ok(Output { endpoint })
	}
}
