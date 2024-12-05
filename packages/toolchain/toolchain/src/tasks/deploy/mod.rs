use anyhow::*;
use rivet_api::{apis, models};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
	build,
	project::environment::TEMPEnvironment,
	ToolchainCtx,
	{config, util::task},
};

mod docker;
mod js;
mod manager;

#[derive(Deserialize)]
pub struct Input {
	pub config: config::Config,
	pub environment_id: Uuid,
	pub build_tags: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct Output {
	pub build_ids: Vec<Uuid>,
}

pub struct Task;

impl task::Task for Task {
	type Input = Input;
	type Output = Output;

	fn name() -> &'static str {
		"deploy"
	}

	async fn run(task: task::TaskCtx, input: Self::Input) -> Result<Self::Output> {
		let ctx = crate::toolchain_ctx::load().await?;

		let env = crate::project::environment::get_env(&ctx, input.environment_id).await?;

		// Reserve version name
		let reserve_res =
			apis::cloud_games_versions_api::cloud_games_versions_reserve_version_name(
				&ctx.openapi_config_cloud,
				&ctx.project.game_id.to_string(),
			)
			.await?;
		let version_name = reserve_res.version_display_name;

		// Manager
		let manager_res = if input.config.unstable().manager.enable() {
			Some(
				manager::deploy(
					&ctx,
					task.clone(),
					manager::DeployOpts {
						env: env.clone(),
						version_name: version_name.clone(),
					},
				)
				.await?,
			)
		} else {
			None
		};

		// Build
		let mut build_ids = Vec::new();
		let mut example_build = None; // Build to use for the example code
		for build in &input.config.builds {
			// Filter out builds that match the tags
			if let Some(filter) = &input.build_tags {
				if !filter.iter().all(|(k, v)| build.tags.get(k) == Some(v)) {
					continue;
				}
			}

			if example_build.is_none() {
				example_build = Some(build);
			}

			// Build
			let build_id = build_and_upload(
				&ctx,
				task.clone(),
				input.config.clone(),
				&env,
				&version_name,
				build,
			)
			.await?;
			build_ids.push(build_id);
		}

		ensure!(!build_ids.is_empty(), "No builds matched build tags");

		task.log("[Deploy Finished]");

		if let Some(manager_res) = &manager_res {
			// Build to use as an example
			let example_build = example_build.context("no example build")?;

			task.log("");
			task.log("Deployed:");
			task.log("");
			task.log("  Actors:          https://hub.rivet.gg/todo");
			task.log("  Builds:          https://hub.rivet.gg/todo");
			task.log(format!("  Endpoint:        {}", manager_res.endpoint));
			task.log("");
			task.log("Next steps:");
			task.log("");
			task.log(r#"  import ActorClient from "@rivet-gg/actors-client";"#);
			task.log(format!(
				r#"  const actorClient = new ActorClient("{}");"#,
				manager_res.endpoint
			));
			task.log("");
			task.log(format!(
				r#"  const actor = await actorClient.withTags({})"#,
				serde_json::to_string(&example_build.tags)?
			));
			task.log(r#"  actor.rpc("myMethod", "Hello, world!")"#);
			task.log("");
		} else {
			task.log("");
			task.log("Deployed:");
			task.log("");
			task.log("  Actors:          https://hub.rivet.gg/todo");
			task.log("  Builds:          https://hub.rivet.gg/todo");
			task.log("");
		}

		Ok(Output { build_ids })
	}
}

/// Builds the required resources and uploads it to Rivet.
///
/// Returns the resulting build ID.
async fn build_and_upload(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	config: config::Config,
	env: &TEMPEnvironment,
	version_name: &str,
	build: &config::Build,
) -> Result<Uuid> {
	// Build tags
	//
	// **version**
	//
	// Unique ident for this build. Used for figuring out which server to start when
	// passing dynamic version from client.
	//
	// **latest**
	//
	// Indicates the latest build to use for this environment. Used if not providing a client-side
	// version.
	// let mut tags = HashMap::from([
	// 	(build::tags::VERSION.to_string(), version_name.to_string()),
	// 	(build::tags::CURRENT.to_string(), "true".to_string()),
	// ]);
	// tags.extend(build.tags.clone());

	// let exclusive_tags = vec![
	// 	build::tags::VERSION.to_string(),
	// 	build::tags::CURRENT.to_string(),
	// ];

	// Build & upload
	let build_id = match &build.runtime {
		config::build::Runtime::Docker(docker) => {
			docker::build_and_upload(
				&ctx,
				task.clone(),
				docker::BuildAndUploadOpts {
					env: env.clone(),
					config: config.clone(),
					tags: build.tags.clone(),
					build_config: docker.clone(),
					version_name: version_name.to_string(),
				},
			)
			.await?
		}
		config::build::Runtime::JavaScript(js) => {
			js::build_and_upload(
				&ctx,
				task.clone(),
				js::BuildAndUploadOpts {
					env: env.clone(),
					tags: build.tags.clone(),
					build_config: js.clone(),
					version_name: version_name.to_string(),
				},
			)
			.await?
		}
	};

	// // Tag build
	// let complete_res = apis::actor_builds_api::actor_builds_patch_tags(
	// 	&ctx.openapi_config_cloud,
	// 	&build_id.to_string(),
	// 	models::ActorPatchBuildTagsRequest {
	// 		tags: Some(serde_json::to_value(&tags)?),
	// 		exclusive_tags: Some(exclusive_tags.clone()),
	// 	},
	// 	Some(&ctx.project.name_id),
	// 	Some(&env.slug),
	// )
	// .await;
	// if let Err(err) = complete_res.as_ref() {
	// 	task.log(format!("{err:?}"));
	// }
	// complete_res.context("complete_res")?;

	// HACK: Multiple exclusive tags doesn't work atm
	{
		let complete_res = apis::actor_builds_api::actor_builds_patch_tags(
			&ctx.openapi_config_cloud,
			&build_id.to_string(),
			models::ActorPatchBuildTagsRequest {
				tags: Some(serde_json::to_value(&build.tags)?),
				exclusive_tags: None,
			},
			Some(&ctx.project.name_id),
			Some(&env.slug),
		)
		.await;
		if let Err(err) = complete_res.as_ref() {
			task.log(format!("{err:?}"));
		}
		complete_res.context("complete_res")?;

		let complete_res = apis::actor_builds_api::actor_builds_patch_tags(
			&ctx.openapi_config_cloud,
			&build_id.to_string(),
			models::ActorPatchBuildTagsRequest {
				tags: Some(serde_json::to_value(&HashMap::from([(
					build::tags::CURRENT.to_string(),
					"true".to_string(),
				)]))?),
				exclusive_tags: Some(vec![build::tags::CURRENT.to_string()]),
			},
			Some(&ctx.project.name_id),
			Some(&env.slug),
		)
		.await;
		if let Err(err) = complete_res.as_ref() {
			task.log(format!("{err:?}"));
		}
		complete_res.context("complete_res")?;

		let complete_res = apis::actor_builds_api::actor_builds_patch_tags(
			&ctx.openapi_config_cloud,
			&build_id.to_string(),
			models::ActorPatchBuildTagsRequest {
				tags: Some(serde_json::to_value(&HashMap::from([(
					build::tags::VERSION.to_string(),
					version_name.to_string(),
				)]))?),
				// TODO: This does not behave correctly atm
				exclusive_tags: None,
				// exclusive_tags: Some(vec![build::tags::VERSION.to_string()]),
			},
			Some(&ctx.project.name_id),
			Some(&env.slug),
		)
		.await;
		if let Err(err) = complete_res.as_ref() {
			task.log(format!("{err:?}"));
		}
		complete_res.context("complete_res")?;
	}

	task.log(format!("[Build Finished] {build_id}"));
	task.log("");

	Ok(build_id)
}
