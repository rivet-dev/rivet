use anyhow::*;
use rivet_api::apis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{config, paths, tasks::build_publish, util::task};

#[derive(Deserialize)]
pub struct Input {
	pub config: config::Config,
	pub environment_id: Uuid,
	pub filter_tags: Option<HashMap<String, String>>,
	pub build_tags: Option<HashMap<String, String>>,
	pub version_name: Option<String>,
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

		// Get version name (provided or reserved)
		let version_name = if let Some(version) = input.version_name {
			version
		} else {
			// Reserve version name if not provided
			let reserve_res =
				apis::cloud_games_versions_api::cloud_games_versions_reserve_version_name(
					&ctx.openapi_config_cloud,
					&ctx.project.game_id.to_string(),
				)
				.await?;
			reserve_res.version_display_name
		};

		// Build
		let mut build_ids = Vec::new();
		let mut example_build = None; // Build to use for the example code
		for (build_name, build) in &input.config.builds {
			// Filter out builds that match the tags
			if let Some(filter) = &input.filter_tags {
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

			// Merge build tags & input tags. Input tags overwrite config tags.
			let build_tags = build
				.tags
				.iter()
				.flatten()
				.chain(input.build_tags.iter().flatten())
				.map(|(k, v)| (k.clone(), v.clone()))
				.collect();

			// Build using build publish task
			let build_id = build_publish::Task::run(
				task.clone(),
				build_publish::Input {
					environment_id: env.id,
					build_tags: Some(build_tags),
					version_name: version_name.clone(),
					build_name: build_name.to_string(),
					runtime: build.runtime.clone(),
				},
			)
			.await?
			.build_id;
			build_ids.push(build_id);
		}

		ensure!(!build_ids.is_empty(), "No builds matched build tags");

		let hub_origin = &ctx.bootstrap.origins.hub;
		let project_slug = &ctx.project.name_id;
		let env_slug = &env.slug;

		task.log("");
		task.log("Deployed:");
		task.log("");
		task.log(format!("  Actors:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/actors"));
		task.log(format!("  Builds:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/builds"));
		task.log("");

		Ok(Output { build_ids })
	}
}
