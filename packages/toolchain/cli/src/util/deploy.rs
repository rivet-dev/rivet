use anyhow::*;
use inquire::{list_option::ListOption, Select};
use serde_json;
use std::collections::HashMap;
use tokio::task::block_in_place;
use toolchain::{
	config, errors,
	rivet_api::{apis, models},
	tasks::{deploy, get_bootstrap_data},
	ToolchainCtx,
};
use uuid::Uuid;

use crate::util::task::{run_task, TaskOutputStyle};

pub struct DeployOpts<'a> {
	pub ctx: &'a ToolchainCtx,
	pub environment: &'a str,
	pub filter_tags: Option<HashMap<String, String>>,
	pub build_tags: Option<HashMap<String, String>>,
	pub version: Option<String>,
}

pub async fn deploy(opts: DeployOpts<'_>) -> Result<Vec<Uuid>> {
	let bootstrap_data =
		run_task::<get_bootstrap_data::Task>(TaskOutputStyle::None, get_bootstrap_data::Input {})
			.await?;
	let Some(cloud_data) = bootstrap_data.cloud else {
		eprintln!("Not signed in. Please run `rivet login`.");
		return Err(errors::GracefulExit.into());
	};

	// Find environment
	let environment = match cloud_data
		.envs
		.iter()
		.find(|env| env.slug == opts.environment)
	{
		Some(env) => env,
		None => {
			eprintln!(
				"Environment '{}' not found. Available environments:",
				opts.environment
			);
			for env in &cloud_data.envs {
				eprintln!("- {}", env.slug);
			}
			return Err(errors::GracefulExit.into());
		}
	};

	let config = toolchain::config::Config::load(None).await?;

	// Upload builds
	let build = run_task::<deploy::Task>(
		TaskOutputStyle::PlainNoResult,
		deploy::Input {
			config: config.clone(),
			environment_id: environment.id,
			filter_tags: opts.filter_tags.clone(),
			build_tags: opts.build_tags.clone(),
			version_name: opts.version.clone(),
		},
	)
	.await?;

	// Setup function routes
	setup_function_routes(opts.ctx, environment, &config, &opts.filter_tags).await?;

	// Print summary
	print_summary(opts.ctx, environment);

	Ok(build.build_ids)
}

async fn setup_function_routes(
	ctx: &ToolchainCtx,
	environment: &toolchain::project::environment::TEMPEnvironment,
	config: &config::Config,
	filter_tags: &Option<HashMap<String, String>>,
) -> Result<()> {
	// Determine default hostname based on project & env
	let default_hostname = format!(
		"{}-{}.{}",
		ctx.project.name_id,
		environment.slug,
		ctx.bootstrap
			.domains
			.job
			.as_ref()
			.context("bootstrap.domains.job")?
	);

	for (fn_name, function) in &config.functions {
		// TODO: Convert this in to a shared fn
		// Filter out builds that match the tags
		if let Some(filter) = &filter_tags {
			if !filter.iter().all(|(k, v)| {
				function.build.full_tags(fn_name).get(k.as_str()) == Some(&v.as_str())
			}) {
				continue;
			}
		}

		// Create route selector tags
		let mut route_tags = HashMap::new();
		route_tags.insert("type".to_string(), "function".to_string());
		route_tags.insert("function".to_string(), fn_name.to_string());

		// Get function config values
		let config_route_subpaths = function.route_subpaths.unwrap_or(true);
		let config_strip_prefix = function.strip_prefix.unwrap_or(true);

		// Check for existing routes matching tags
		let routes_response = apis::routes_api::routes_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id.to_string()),
			Some(&environment.slug),
		)
		.await?;

		// Find routes that match this function name
		let matching_route = routes_response
			.routes
			.iter()
			.find(|route| route.id == *fn_name);

		if let Some(matching_route) = matching_route {
			// Check if any route matches our config exactly
			let exact_match = matching_route.path == function.path()
				&& matching_route.route_subpaths == config_route_subpaths
				&& matching_route.strip_prefix == config_strip_prefix
				&& matching_route
					.target
					.actors
					.as_ref()
					.map_or(false, |actors| actors.selector_tags == route_tags);

			if exact_match {
				// Skip if an exact match is found
				// println!("Route already exists for function '{}'", fn_name);
				continue;
			}

			// Get existing selector tags
			let existing_selector_tags = matching_route
				.target
				.actors
				.as_ref()
				.map(|actors| &actors.selector_tags);

			// Build a dynamic list of changes
			let mut changes = Vec::new();
			if matching_route.path != function.path() {
				changes.push(format!(
					"Path: '{}' → '{}'",
					matching_route.path, function.path()
				));
			}
			if matching_route.route_subpaths != config_route_subpaths {
				changes.push(format!(
					"Route subpaths: {} → {}",
					matching_route.route_subpaths, config_route_subpaths
				));
			}
			if matching_route.strip_prefix != config_strip_prefix {
				changes.push(format!(
					"Strip prefix: {} → {}",
					matching_route.strip_prefix, config_strip_prefix
				));
			}
			if let Some(existing_tags) = existing_selector_tags {
				if *existing_tags != route_tags {
					let existing_json = serde_json::to_string(existing_tags).unwrap_or_default();
					let new_json = serde_json::to_string(&route_tags).unwrap_or_default();
					changes.push(format!("Selector tags: {} → {}", existing_json, new_json));
				}
			}

			// Format all changes with bullet points
			let changes_text = changes
				.iter()
				.map(|change| format!("\n  - {}", change))
				.collect::<String>();

			let options = &[
				ListOption::new(0, "Sync route with config"),
				ListOption::new(1, "Keep existing route"),
			];

			println!();
			let choice = block_in_place(|| {
				Select::new(
					&format!(
						"Route configuration for '{fn_name}' has changed{}",
						changes_text
					),
					options.to_vec(),
				)
				.with_starting_cursor(0)
				.prompt()
			})?;

			match choice.index {
				0 => {
					// Update first matching route to match config
					let mut update_route_body = models::RoutesUpdateRouteBody {
						hostname: matching_route.hostname.clone(),
						path: matching_route.path.clone(),
						route_subpaths: matching_route.route_subpaths,
						strip_prefix: matching_route.strip_prefix,
						target: Box::new(models::RoutesRouteTarget {
							actors: Some(Box::new(models::RoutesRouteTargetActors {
								selector_tags: route_tags.clone(),
							})),
						}),
					};

					// Only update fields that have changed
					if matching_route.path != function.path() {
						update_route_body.path = function.path();
					}
					if matching_route.route_subpaths != config_route_subpaths {
						update_route_body.route_subpaths = config_route_subpaths;
					}
					if matching_route.strip_prefix != config_strip_prefix {
						update_route_body.strip_prefix = config_strip_prefix;
					}

					let result = apis::routes_api::routes_update(
						&ctx.openapi_config_cloud,
						&matching_route.id,
						update_route_body.clone(),
						Some(&ctx.project.name_id.to_string()),
						Some(&environment.slug),
					)
					.await;

					match result {
						Result::Ok(_) => {
							println!(
								"Successfully updated route: {}{}",
								update_route_body.hostname, update_route_body.path
							);
						}
						Err(err) => {
							eprintln!("Failed to update route: {}", err);
						}
					}
				}
				1 => {
					// Do nothing
					println!("Ignoring route configuration differences");
				}
				_ => unreachable!(),
			}
		} else {
			let options = &[
				ListOption::new(
					0,
					format!(
						"Create default route ({default_hostname}{path})",
						path = function.path()
					),
				),
				ListOption::new(1, "Skip route creation".to_string()),
			];

			println!();
			let choice = block_in_place(|| {
				Select::new(
					&format!("Set up routing for function '{}':", fn_name),
					options.to_vec(),
				)
				.with_help_message("Routes can be manually created in the Rivet dashboard")
				.with_starting_cursor(0)
				.prompt()
			})?;

			match choice.index {
				0 => {
					// Create route with default settings
					create_function_route(
						ctx,
						environment,
						fn_name,
						function,
						&route_tags,
						&default_hostname,
					)
					.await?;
				}
				1 => {
					// Skip creating a route
					continue;
				}
				_ => unreachable!(),
			}
		}
	}

	Ok(())
}

fn print_summary(ctx: &ToolchainCtx, env: &toolchain::project::environment::TEMPEnvironment) {
	let hub_origin = &ctx.bootstrap.origins.hub;
	let project_slug = &ctx.project.name_id;
	let env_slug = &env.slug;

	println!("");
	println!("Deployed:");
	println!("");
	println!(
		"  Actors:          {hub_origin}/projects/{project_slug}/environments/{env_slug}/actors"
	);
	println!("  Containers:      {hub_origin}/projects/{project_slug}/environments/{env_slug}/containers");
	println!(
		"  Functions:       {hub_origin}/projects/{project_slug}/environments/{env_slug}/functions"
	);
	println!(
		"  Logs:            {hub_origin}/projects/{project_slug}/environments/{env_slug}/logs"
	);
	println!("  Version:         {hub_origin}/projects/{project_slug}/environments/{env_slug}/actor-versions");
	println!("");
}

async fn create_function_route(
	ctx: &ToolchainCtx,
	environment: &toolchain::project::environment::TEMPEnvironment,
	fn_name: &str,
	function: &config::Function,
	route_tags: &HashMap<String, String>,
	default_hostname: &str,
) -> Result<()> {
	// Get route_subpaths and strip_prefix from config
	let default_route_subpaths = function.route_subpaths.unwrap_or(true);
	let default_strip_prefix = function.strip_prefix.unwrap_or(true);

	// Loop until route creation succeeds
	let mut route_created = false;
	while !route_created {
		let hostname = default_hostname.to_string();
		let path = function.path();
		let route_subpaths = default_route_subpaths;
		let strip_prefix = default_strip_prefix;

		// Prepare route body
		let update_route_body = models::RoutesUpdateRouteBody {
			hostname,
			path,
			route_subpaths,
			strip_prefix,
			target: Box::new(models::RoutesRouteTarget {
				actors: Some(Box::new(models::RoutesRouteTargetActors {
					selector_tags: route_tags.clone(),
				})),
			}),
		};

		// Create/update route
		let result = apis::routes_api::routes_update(
			&ctx.openapi_config_cloud,
			&fn_name,
			update_route_body.clone(),
			Some(&ctx.project.name_id.to_string()),
			Some(&environment.slug),
		)
		.await;

		match result {
			Result::Ok(_) => {
				println!(
					"Successfully created route: {}{}",
					update_route_body.hostname, update_route_body.path
				);
				route_created = true;
			}
			Err(err) => {
				eprintln!("Failed to create route: {}", err);
				break;
			}
		}
	}

	Ok(())
}
