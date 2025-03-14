use anyhow::*;
use rivet_api::apis;
use serde::{Deserialize, Serialize};

use crate::util::task;

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
		let res = apis::actors_api::actors_list(
			&ctx.openapi_config_actor,
			Some(&ctx.project.name_id),
			Some(&input.env_slug),
			None,
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

		let endpoint = crate::util::actor_manager::extract_endpoint(&actor)?;

		Ok(Output { endpoint })
	}
}
