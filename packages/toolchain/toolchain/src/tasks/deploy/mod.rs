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
						manager_config: input.config.unstable().manager,
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
		for (build_name, build) in &input.config.builds {
			// Filter out builds that match the tags
			if let Some(filter) = &input.build_tags {
				if !filter
					.iter()
					.all(|(k, v)| build.full_tags(build_name).get(k.as_str()) == Some(&v.as_str()))
				{
					continue;
				}
			}

			if example_build.is_none() {
				example_build = Some((build_name, build));
			}

			// Build
			let build_id = build_and_upload(
				&ctx,
				task.clone(),
				input.config.clone(),
				&env,
				&version_name,
				build_name,
				build,
			)
			.await?;
			build_ids.push(build_id);
		}

		ensure!(!build_ids.is_empty(), "No builds matched build tags");

		task.log("[Deploy Finished]");

		if let Some(manager_res) = &manager_res {
			// Build to use as an example
			let (example_build_name, _example_build) = example_build.context("no example build")?;

			let hub_origin = &ctx.bootstrap.origins.hub;
			let project_id = ctx.project.game_id;
			let env_id = env.id;
			task.log("");
			task.log("Deployed:");
			task.log("");
			task.log(format!("  Actors:          {hub_origin}/projects/{project_id}/environments/{env_id}/actors"));
			task.log(format!("  Builds:          {hub_origin}/projects/{project_id}/environments/{env_id}/builds"));
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
				r#"  const actor = await actorClient.get({{ name: "{example_build_name}" }})"#,
			));
			task.log(r#"  actor.myRpc("Hello, world!");"#);
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
	build_name: &str,
	build: &config::Build,
) -> Result<Uuid> {
	task.log("");

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
	let build_tags = build
		.full_tags(build_name)
		.into_iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect::<HashMap<_, _>>();
	let build_id = match &build.runtime {
		config::build::Runtime::Docker(docker) => {
			docker::build_and_upload(
				&ctx,
				task.clone(),
				docker::BuildAndUploadOpts {
					env: env.clone(),
					config: config.clone(),
					tags: build_tags.clone(),
					build_config: docker.clone(),
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
					tags: build_tags.clone(),
					build_config: js.clone(),
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
				tags: Some(serde_json::to_value(&build_tags)?),
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
				tags: Some(serde_json::json!({
					build::tags::ACCESS: build.access,
				})),
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

	// Upgrade actors
	task.log(format!("[Upgrading Actors]"));
	apis::actor_api::actor_upgrade_all(
		&ctx.openapi_config_cloud,
		models::ActorUpgradeAllActorsRequest {
			tags: Some(serde_json::to_value(&build_tags)?),
			build: Some(build_id),
			build_tags: None,
		},
		Some(&ctx.project.name_id),
		Some(&env.slug),
	)
	.await?;

	task.log(format!("[Build Finished] {build_id}"));
	task.log("");

	Ok(build_id)
}
