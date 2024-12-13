use anyhow::*;
use rivet_api::{apis, models};
use std::collections::HashMap;

use crate::{
	config, paths, project::environment::TEMPEnvironment, toolchain_ctx::ToolchainCtx, util::task,
};

pub struct DeployOpts {
	pub env: TEMPEnvironment,
	pub manager_config: config::ManagerUnstable,
}

pub struct DeployOutput {
	pub endpoint: String,
}

pub async fn deploy(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	opts: DeployOpts,
) -> Result<DeployOutput> {
	// Get source
	let manager_src_path = rivet_actors_sdk_embed::src_path(&paths::data_dir()?).await?;

	let tags = HashMap::from([
		("name".to_string(), "manager".to_string()),
		("owner".to_string(), "rivet".to_string()),
	]);

	// Deploy manager
	let build_id = super::js::build_and_upload(
		ctx,
		task.clone(),
		super::js::BuildAndUploadOpts {
			env: opts.env.clone(),
			tags: tags.clone(),
			build_config: config::build::javascript::Build {
				script: manager_src_path
					.join("manager")
					.join("src")
					.join("mod.ts")
					.display()
					.to_string(),
				bundler: Some(config::build::javascript::Bundler::Deno),
				deno: config::build::javascript::Deno {
					// TODO(RVT-4382): Does not support workspaces, so we have to point to the
					// manager's Deno config
					config_path: Some(
						manager_src_path
							.join("manager")
							.join("deno.jsonc")
							.display()
							.to_string(),
					),
					import_map_url: None,
					lock_path: Some(manager_src_path.join("deno.lock").display().to_string()),
				},
				unstable: opts.manager_config.js_unstable.clone(),
			},
		},
	)
	.await?;

	// Check if manager exists
	let res = apis::actor_api::actor_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(&opts.env.slug),
		None,
		Some(&serde_json::to_string(&serde_json::json!({
			"name": "manager",
		}))?),
		Some(false),
		None,
	)
	.await?;
	if res.actors.len() > 1 {
		eprintln!("WARNING: More than 1 manager actor is running. We recommend manually stopping one of them.")
	}
	let actor = if let Some(actor) = res.actors.into_iter().next() {
		// Upgrade manager actor
		apis::actor_api::actor_upgrade(
			&ctx.openapi_config_cloud,
			&actor.id.to_string(),
			models::ActorUpgradeActorRequest {
				build: Some(build_id),
				build_tags: None,
			},
			Some(&ctx.project.name_id),
			Some(&opts.env.slug),
		)
		.await?;

		actor
	} else {
		// Create new actor

		// Choose a region that's closest to the core Rivet datacenter. This actor makes a lot of API
		// requests to Rivet, so we want to reduce that RTT.
		let regions = apis::actor_regions_api::actor_regions_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id),
			Some(&opts.env.slug),
		)
		.await?;
		let region = if let Some(ideal_region) = regions
			.regions
			.iter()
			.filter(|r| r.id == "atl" || r.id == "local")
			.next()
		{
			ideal_region.id.clone()
		} else {
			regions.regions.first().context("no regions")?.id.clone()
		};

		// Issue service token
		let service_token =
			apis::games_environments_tokens_api::games_environments_tokens_create_service_token(
				&ctx.openapi_config_cloud,
				&ctx.project.game_id.to_string(),
				&opts.env.id.to_string(),
			)
			.await?;

		// TODO(RVT-4263): Auto-determine TCP or HTTP networking
		// Get or create actor
		let request = models::ActorCreateActorRequest {
			region: Some(region),
			tags: Some(serde_json::json!(tags)),
			build: Some(build_id),
			build_tags: None,
			runtime: Some(Box::new(models::ActorCreateActorRuntimeRequest {
				environment: Some(HashMap::from([(
					"RIVET_SERVICE_TOKEN".to_string(),
					service_token.token,
				)])),
			})),
			network: Some(Box::new(models::ActorCreateActorNetworkRequest {
				mode: Some(models::ActorNetworkMode::Bridge),
				ports: Some(HashMap::from([(
					crate::util::actor_manager::HTTP_PORT.to_string(),
					models::ActorCreateActorPortRequest {
						protocol: models::ActorPortProtocol::Https,
						internal_port: None,
						routing: Some(Box::new(models::ActorPortRouting {
							host: None,
							guard: Some(serde_json::json!({})),
						})),
					},
				)])),
			})),
			resources: None,
			lifecycle: Some(Box::new(models::ActorLifecycle {
				durable: Some(true),
				kill_timeout: None,
			})),
		};
		let response = apis::actor_api::actor_create(
			&ctx.openapi_config_cloud,
			request,
			Some(&ctx.project.name_id),
			Some(&opts.env.slug),
			None,
		)
		.await?;

		*response.actor
	};

	let endpoint = crate::util::actor_manager::extract_endpoint(&actor)?;

	Ok(DeployOutput { endpoint })
}
