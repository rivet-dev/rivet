use anyhow::*;
use rivet_api::{apis, models};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
	config, paths,
	tasks::build_publish,
	util::task::{self, Task as _},
};

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
		let version_name = if let Some(ref version) = input.version_name {
			version.clone()
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
		let build_ids =
			perform_builds(&task, &input, &ctx, env.id, &env.slug, &version_name).await?;

		// Create edge function actors
		create_edge_function_actors(&task, &ctx, &input, &env.slug, &version_name).await?;

		Ok(Output { build_ids })
	}
}

async fn perform_builds(
	task: &task::TaskCtx, // Currently unused
	input: &Input,
	_ctx: &crate::toolchain_ctx::ToolchainCtx, // Currently unused
	environment_id: Uuid,
	_environment_slug: &str, // Currently unused
	version_name: &str,
) -> Result<Vec<Uuid>> {
	let mut build_ids = Vec::new();
	let builds_iter = input
		.config
		.actors
		.iter()
		.map(|(k, v)| (k, &v.build, "actor"))
		.chain(
			input
				.config
				.containers
				.iter()
				.map(|(k, v)| (k, &v.build, "container")),
		)
		.chain(
			input
				.config
				.functions
				.iter()
				.map(|(k, v)| (k, &v.build, "function")),
		);
	for (build_name, build, type_value) in builds_iter {
		// Filter out builds that match the tags
		if let Some(filter) = &input.filter_tags {
			if !filter
				.iter()
				.all(|(k, v)| build.full_tags(build_name).get(k.as_str()) == Some(&v.as_str()))
			{
				continue;
			}
		}

		// Merge build tags & input tags. Input tags overwrite config tags.
		let mut build_tags: HashMap<String, String> = build
			.tags
			.iter()
			.flatten()
			.chain(input.build_tags.iter().flatten())
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect();
		build_tags.insert("type".into(), type_value.into());
		build_tags.insert("name".into(), build_name.to_string());
		build_tags.insert("current".into(), "true".to_string());

		// Build using build publish task
		let output = build_publish::Task::run(
			task.clone(),
			build_publish::Input {
				environment_id,
				build_tags: Some(build_tags),
				version_name: version_name.to_string(),
				build_name: build_name.to_string(),
				runtime: build.runtime.clone(),
			},
		)
		.await?;
		build_ids.push(output.build_id);
	}

	ensure!(!build_ids.is_empty(), "No builds matched build tags");
	Ok(build_ids)
}

async fn create_edge_function_actors(
	_task: &task::TaskCtx, // Used for logging
	ctx: &crate::toolchain_ctx::ToolchainCtx,
	input: &Input,
	environment_slug: &str,
	_version_name: &str, // Currently unused but kept for consistency
) -> Result<()> {
	// Skip if no functions
	if input.config.functions.is_empty() {
		return Ok(());
	}

	// Get all available regions
	let regions_res = apis::regions_api::regions_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(environment_slug),
	)
	.await
	.context("Failed to list regions")?;

	let region_ids: Vec<String> = regions_res.regions.iter().map(|r| r.id.clone()).collect();

	// Process each function
	for (fn_name, function) in &input.config.functions {
		// Filter out functions that don't match the tags
		if let Some(filter) = &input.filter_tags {
			if !filter.iter().all(|(k, v)| {
				function.build.full_tags(fn_name).get(k.as_str()) == Some(&v.as_str())
			}) {
				continue;
			}
		}

		// Define actor tags for this function
		let actor_tags = json!({
			"type": "function",
			"function": fn_name,
		});

		// List all existing actors for this function
		let actors_res = apis::actors_api::actors_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id),
			Some(environment_slug),
			None,
			Some(&serde_json::to_string(&actor_tags)?),
			Some(false), // Don't include destroyed actors
			None,
		)
		.await
		.context("Failed to list actors")?;

		// Track which regions already have actors
		let mut existing_regions = HashSet::new();
		let mut existing_actors = HashMap::new();

		for actor in &actors_res.actors {
			existing_regions.insert(actor.region.clone());
			existing_actors.insert(actor.region.clone(), actor.id);
		}

		// Use build tags to match the appropriate build
		// This is more robust than using a specific build ID
		let build_tags = json!({
			"name": fn_name,
			"current": "true",
			"type": "function"
		});

		// Create or upgrade actors for each region
		for region in &region_ids {
			if existing_regions.contains(region) {
				// Upgrade existing actor
				let actor_id = existing_actors.get(region).unwrap();
				// task.log("");
				// task.log(&format!("[{fn_name}] Upgrading function in {region}"));

				apis::actors_api::actors_upgrade(
					&ctx.openapi_config_cloud,
					&actor_id.to_string(),
					models::ActorsUpgradeActorRequest {
						build: None, // Use build tags instead
						build_tags: Some(Some(build_tags.clone())),
					},
					Some(&ctx.project.name_id),
					Some(environment_slug),
				)
				.await
				.context(format!("Failed to upgrade actor in region {}", region))?;
			} else {
				// Create new actor
				// task.log("");
				// task.log(&format!("[{fn_name}] Setting up function in {region}"));

				// Create actor request
				let (resources, internal_port) = match &function.build.runtime {
					crate::config::build::Runtime::Docker(_) => {
						// Configure resources & networking
						let resources = function.resources();
						(
							Some(Box::new(models::ActorsResources {
								cpu: resources.cpu as i32,
								memory: resources.memory as i32,
							})),
							Some(function.networking.internal_port() as i32),
						)
					}
					crate::config::build::Runtime::JavaScript(_) => {
						// Isolates don't support resources & internal port
						(None, None)
					}
				};

				let create_actor_request = models::ActorsCreateActorRequest {
					region: Some(region.clone()),
					tags: Some(actor_tags.clone()),
					build: None,
					build_tags: Some(Some(build_tags.clone())),
					runtime: Some(Box::new(models::ActorsCreateActorRuntimeRequest {
						environment: None,
						network: None,
					})),
					network: Some(Box::new(models::ActorsCreateActorNetworkRequest {
						mode: Some(models::ActorsNetworkMode::Bridge),
						ports: Some(HashMap::from([(
							"http".to_string(),
							models::ActorsCreateActorPortRequest {
								protocol: models::ActorsPortProtocol::Https,
								routing: Some(Box::new(models::ActorsPortRouting {
									guard: Some(json!({})),
									host: None,
								})),
								internal_port,
							},
						)])),
						wait_ready: None,
					})),
					resources,
					lifecycle: Some(Box::new(models::ActorsLifecycle {
						durable: Some(true),
						kill_timeout: None,
					})),
				};

				apis::actors_api::actors_create(
					&ctx.openapi_config_cloud,
					create_actor_request,
					Some(&ctx.project.name_id),
					Some(environment_slug),
					None,
				)
				.await
				.context(format!("Failed to create actor in region {}", region))?;
			}
		}
	}

	Ok(())
}
