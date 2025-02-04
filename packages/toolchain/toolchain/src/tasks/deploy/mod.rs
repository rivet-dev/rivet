use anyhow::*;
use rivet_api::{apis, models};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
	build, paths,
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

		// Check for deno.json or deno.jsonc
		let project_root = paths::project_root()?;
		if project_root.join("deno.json").exists() || project_root.join("deno.jsonc").exists() {
			task.log("[WARNING] deno.json and deno.jsonc are not supported at the moment. Please use package.json with NPM instead.");
		}

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

		let hub_origin = &ctx.bootstrap.origins.hub;
		let project_slug = &ctx.project.name_id;
		let env_slug = &env.slug;

		if let Some(manager_res) = &manager_res {
			// Build to use as an example
			let (example_build_name, _example_build) = example_build.context("no example build")?;

			task.log("");
			task.log("Deployed:");
			task.log("");
			task.log(format!("  Actors:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/actors"));
			task.log(format!("  Builds:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/builds"));
			task.log(format!("  Endpoint:        {}", manager_res.endpoint));
			task.log("");
			task.log("Connect to your actor:");
			task.log("");
			task.log(r#"  import ActorClient from "@rivet-gg/actor-client";"#);
			task.log(format!(
				r#"  const actorClient = new ActorClient("{}");"#,
				manager_res.endpoint
			));
			task.log(format!(
				r#"  const actor = await actorClient.get({{ name: "{example_build_name}" }})"#,
			));
			task.log(r#"  actor.myRpc("Hello, world!");"#);
			task.log("");
		} else {
			task.log("");
			task.log("Deployed:");
			task.log("");
			task.log(format!("  Actors:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/actors"));
			task.log(format!("  Builds:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/builds"));
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

	let mut tags = HashMap::from([
		(build::tags::NAME.to_string(), build_name.to_string()),
		(
			build::tags::ACCESS.to_string(),
			build.access.as_ref().to_string(),
		),
		(build::tags::VERSION.to_string(), version_name.to_string()),
		(build::tags::CURRENT.to_string(), "true".to_string()),
	]);
	if let Some(build_tags) = build.tags.clone() {
		tags.extend(build_tags.clone());
	}

	// Find existing builds with current tag
	let list_res = apis::actor_builds_api::actor_builds_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(&env.slug),
		Some(&serde_json::to_string(&json!({
			build::tags::NAME: build_name,
			build::tags::CURRENT: "true",
		}))?),
	)
	.await?;

	// Remove current tag if needed
	for build in list_res.builds {
		apis::actor_builds_api::actor_builds_patch_tags(
			&ctx.openapi_config_cloud,
			&build.id.to_string(),
			models::ActorPatchBuildTagsRequest {
				tags: Some(serde_json::to_value(&json!({
					build::tags::CURRENT: null
				}))?),
				exclusive_tags: None,
			},
			Some(&ctx.project.name_id),
			Some(&env.slug),
		)
		.await?;
	}

	// Tag build
	let complete_res = apis::actor_builds_api::actor_builds_patch_tags(
		&ctx.openapi_config_cloud,
		&build_id.to_string(),
		models::ActorPatchBuildTagsRequest {
			tags: Some(serde_json::to_value(&tags)?),
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
