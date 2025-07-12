use anyhow::*;
use indoc::formatdoc;
use inquire::{list_option::ListOption, Select};
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tempfile;
use tokio::task::block_in_place;
use toolchain::{
	config, errors,
	rivet_api::{apis, models},
	tasks::{deploy, get_bootstrap_data},
	ToolchainCtx,
};
use uuid::Uuid;

use crate::util::{
	self,
	task::{run_task, TaskOutputStyle},
};

#[derive(Debug)]
struct PackageManager {
	name: String,
	install_cmd: String,
	install_prod_cmd: String,
	copy_files: Vec<String>,
	cache_mount: String,
}

fn detect_package_manager(project_root: &Path) -> PackageManager {
	// Check for lockfiles in order of preference
	if project_root.join("yarn.lock").exists() {
		PackageManager {
			name: "yarn".to_string(),
			install_cmd: "yarn install --frozen-lockfile".to_string(),
			install_prod_cmd: "yarn install --production --frozen-lockfile".to_string(),
			copy_files: vec!["package.json".to_string(), "yarn.lock".to_string()],
			cache_mount: "--mount=type=cache,id=yarn,target=/usr/local/share/.cache/yarn"
				.to_string(),
		}
	} else if project_root.join("bun.lockb").exists() || project_root.join("bun.lock").exists() {
		// Determine which bun lockfile exists and use appropriate copy files
		let lockfile = if project_root.join("bun.lockb").exists() {
			"bun.lockb"
		} else {
			"bun.lock"
		};
		PackageManager {
			name: "bun".to_string(),
			install_cmd: "bun install --frozen-lockfile".to_string(),
			install_prod_cmd: "bun install --production --frozen-lockfile".to_string(),
			copy_files: vec!["package.json".to_string(), lockfile.to_string()],
			cache_mount: "--mount=type=cache,id=bun,target=/root/.bun".to_string(),
		}
	} else if project_root.join("pnpm-lock.yaml").exists() {
		PackageManager {
			name: "pnpm".to_string(),
			install_cmd: "pnpm install --frozen-lockfile".to_string(),
			install_prod_cmd: "pnpm install --production --frozen-lockfile".to_string(),
			copy_files: vec!["package.json".to_string(), "pnpm-lock.yaml".to_string()],
			cache_mount: "--mount=type=cache,id=pnpm,target=/root/.pnpm-store".to_string(),
		}
	} else if project_root.join("package-lock.json").exists() {
		PackageManager {
			name: "npm".to_string(),
			install_cmd: "npm ci".to_string(),
			install_prod_cmd: "npm ci --omit=dev".to_string(),
			copy_files: vec!["package.json".to_string(), "package-lock.json".to_string()],
			cache_mount: "--mount=type=cache,id=npm,target=/root/.npm".to_string(),
		}
	} else {
		// Default to npm if no lockfile is found
		PackageManager {
			name: "npm".to_string(),
			install_cmd: "npm install".to_string(),
			install_prod_cmd: "npm install --production".to_string(),
			copy_files: vec!["package.json".to_string()],
			cache_mount: "--mount=type=cache,id=npm,target=/root/.npm".to_string(),
		}
	}
}

pub struct DeployOpts<'a> {
	pub ctx: &'a ToolchainCtx,
	pub environment: &'a str,
	pub filter_tags: Option<HashMap<String, String>>,
	pub build_tags: Option<HashMap<String, String>>,
	pub version: Option<String>,
	pub skip_route_creation: Option<bool>,
	pub keep_existing_routes: Option<bool>,
	pub non_interactive: bool,
	pub upgrade: bool,
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

	// Load config
	let mut root = toolchain::config::Config::load(None).await?;

	// Setup Rivetkit
	let mut _rivet_actor_tempfiles = Vec::new();
	let mut _rivet_actor_tempdirs = Vec::new();
	if let Some(rivetkit) = &root.rivetkit {
		// Create service token
		let service_token =
			apis::games_environments_tokens_api::games_environments_tokens_create_service_token(
				&opts.ctx.openapi_config_cloud,
				&opts.ctx.project.game_id.to_string(),
				&environment.id.to_string(),
			)
			.await?;

		// Add server
		let mut server_tags = rivetkit.tags.clone().unwrap_or_default();
		server_tags.insert("role".into(), "server".into());
		server_tags.insert("framework".into(), "rivetkit".into());

		let mut env_vars = rivetkit
			.runtime
			.environment
			.iter()
			.flat_map(|x| x.clone().into_iter())
			.collect::<HashMap<String, String>>();
		env_vars.insert("RIVETKIT_DRIVER".into(), "rivet".into());
		env_vars.insert("RIVET_ENDPOINT".into(), cloud_data.api_endpoint);
		env_vars.insert("RIVET_SERVICE_TOKEN".into(), service_token.token);
		env_vars.insert("RIVET_PROJECT".into(), opts.ctx.project.name_id.clone());
		env_vars.insert("RIVET_ENVIRONMENT".into(), environment.slug.clone());

		// Auto-generate actor enterypoint
		//
		// Has to be in the project path in order to use NPM dependencies
		let project_root = toolchain::paths::project_root()?;
		let actor_tempfile = tempfile::Builder::new()
			.prefix("rivet-actor-")
			.suffix(".tmp.ts")
			.tempfile_in(&project_root)?;
		let actor_path = actor_tempfile.path().to_owned();
		_rivet_actor_tempfiles.push(actor_tempfile);

		let registry_abs_path = project_root.join(&rivetkit.registry);
		let registry_display_path = registry_abs_path.display();
		tokio::fs::write(
			&actor_path,
			generate_actor_script(registry_display_path.to_string()),
		)
		.await?;

		// Determine server runtime
		let function_build = if rivetkit.build.image.is_some()
			|| rivetkit.build.dockerfile.is_some()
		{
			// Use user-provided Docker config
			rivetkit.build.clone()
		} else {
			// Auto-generate server Dockerfile
			//
			// Has to be in the project path in order to use NPM dependencies
			let dockerfile_tempdir = tempfile::Builder::new().prefix("rivet-server-").tempdir()?;
			let dockerfile_path = dockerfile_tempdir.path().join("Dockerfile");
			let dockerignore_path = dockerfile_tempdir.path().join("Dockerfile.dockerignore");
			_rivet_actor_tempdirs.push(dockerfile_tempdir);

			let server_path = rivetkit.server.clone();
			tokio::fs::write(
				&dockerfile_path,
				generate_server_dockerfile(&project_root, server_path),
			)
			.await?;
			tokio::fs::write(&dockerignore_path, generate_server_dockerignore()).await?;

			toolchain::config::build::docker::Build {
				dockerfile: Some(dockerfile_path.display().to_string()),
				build_path: Some(project_root.display().to_string()),
				image: None,
				build_target: None,
				build_args: None,
				unstable: rivetkit.build.unstable.clone(),
			}
		};

		// Add function
		root.functions.insert(
			crate::util::rivetkit::SERVER_NAME.into(),
			toolchain::config::Function {
				build: toolchain::config::Build {
					tags: Some(server_tags),
					runtime: toolchain::config::build::Runtime::Docker(function_build),
				},
				path: rivetkit.path.clone(),
				route_subpaths: rivetkit.route_subpaths,
				strip_prefix: rivetkit.strip_prefix,
				networking: rivetkit.networking.clone(),
				runtime: toolchain::config::FunctionRuntime {
					environment: Some(env_vars),
				},
				resources: rivetkit.resources.clone(),
			},
		);

		// Add actor
		let mut actor_tags = rivetkit.tags.clone().unwrap_or_default();
		actor_tags.insert("role".into(), "actor".into());
		actor_tags.insert("framework".into(), "rivetkit".into());

		root.actors.insert(
			crate::util::rivetkit::ACTOR_NAME.into(),
			toolchain::config::Actor {
				build: toolchain::config::Build {
					tags: Some(actor_tags),
					runtime: toolchain::config::build::Runtime::JavaScript(
						toolchain::config::build::javascript::Build {
							script: actor_path.display().to_string(),
							unstable: toolchain::config::build::javascript::Unstable::default(),
						},
					),
				},
			},
		);
	}

	let config = toolchain::config::Config(Arc::new(root));

	// Upload builds
	let build = run_task::<deploy::Task>(
		TaskOutputStyle::PlainNoResult,
		deploy::Input {
			config: config.clone(),
			environment_id: environment.id,
			filter_tags: opts.filter_tags.clone(),
			build_tags: opts.build_tags.clone(),
			version_name: opts.version.clone(),
			upgrade: opts.upgrade,
		},
	)
	.await?;

	// Setup function routes
	let routes_output = setup_function_routes(
		opts.ctx,
		environment,
		&config,
		&opts.filter_tags,
		opts.skip_route_creation,
		opts.keep_existing_routes,
		opts.non_interactive,
	)
	.await?;

	// Print summary
	print_summary(opts.ctx, environment, config.clone(), routes_output);

	Ok(build.build_ids)
}

struct SetupRoutesOutput {
	rivetkit_endpoint: Option<String>,
}

async fn setup_function_routes(
	ctx: &ToolchainCtx,
	environment: &toolchain::project::environment::TEMPEnvironment,
	config: &config::Config,
	filter_tags: &Option<HashMap<String, String>>,
	skip_route_creation: Option<bool>,
	keep_existing_routes: Option<bool>,
	non_interactive: bool,
) -> Result<SetupRoutesOutput> {
	let mut rivetkit_endpoint = None;
	for (fn_name, function) in &config.functions {
		let is_rivetkit = fn_name == util::rivetkit::SERVER_NAME;

		// Determine default hostname based on project & env
		let default_hostname = if is_rivetkit {
			// Don't include fn name to keep the endpoint clean
			format!(
				"{}-{}.{}",
				ctx.project.name_id,
				environment.slug,
				ctx.bootstrap
					.domains
					.job
					.as_ref()
					.context("bootstrap.domains.job")?
			)
		} else {
			// Include fn name in case there are multiple functions
			format!(
				"{}-{}-{fn_name}.{}",
				ctx.project.name_id,
				environment.slug,
				ctx.bootstrap
					.domains
					.job
					.as_ref()
					.context("bootstrap.domains.job")?
			)
		};

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

				if is_rivetkit {
					rivetkit_endpoint = Some(matching_route.hostname.clone());
				}

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
					matching_route.path,
					function.path()
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

			let choice_index = if non_interactive {
				// In non-interactive mode, use auto_sync_routes if provided, otherwise sync by default
				match keep_existing_routes {
					Some(true) => {
						println!("Skipping route sync for '{fn_name}' (non-interactive mode)");
						1
					}
					Some(false) => {
						println!("Auto-syncing route configuration for '{fn_name}' (non-interactive mode)");
						0
					}
					None => {
						println!("Auto-syncing route configuration for '{fn_name}' (non-interactive mode)");
						0
					}
				}
			} else {
				// Interactive mode - prompt the user
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
				choice.index
			};

			match choice_index {
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

							if is_rivetkit {
								rivetkit_endpoint = Some(default_hostname);
							}
						}
						Err(err) => {
							bail!("Failed to update route: {}", err);
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

			let choice_index = if is_rivetkit {
				println!("Creating route for RietKit");
				0
			} else if non_interactive {
				// In non-interactive mode, use auto_create_routes if provided, otherwise create by default
				match skip_route_creation {
					Some(true) => {
						println!("Skipping route creation for '{fn_name}' (non-interactive mode)");
						1
					}
					Some(false) => {
						println!(
							"Auto-creating route for function '{fn_name}' (non-interactive mode)"
						);
						0
					}
					None => {
						println!(
							"Auto-creating route for function '{fn_name}' (non-interactive mode)"
						);
						0
					}
				}
			} else {
				// Interactive mode - prompt the user
				let choice = block_in_place(|| {
					Select::new(
						&format!("Set up routing for function '{}':", fn_name),
						options.to_vec(),
					)
					.with_help_message("Routes can be manually created in the Rivet dashboard")
					.with_starting_cursor(0)
					.prompt()
				})?;
				choice.index
			};

			match choice_index {
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

					if is_rivetkit {
						rivetkit_endpoint = Some(default_hostname);
					}
				}
				1 => {
					// Skip creating a route
					continue;
				}
				_ => unreachable!(),
			}
		}
	}

	Ok(SetupRoutesOutput { rivetkit_endpoint })
}

fn print_summary(
	ctx: &ToolchainCtx,
	env: &toolchain::project::environment::TEMPEnvironment,
	config: config::Config,
	routes_output: SetupRoutesOutput,
) {
	let hub_origin = &ctx.bootstrap.origins.hub;
	let project_slug = &ctx.project.name_id;
	let env_slug = &env.slug;

	println!("");
	println!("Deploy Success:");
	println!("");
	if config.rivetkit.is_some() {
		if let Some(endpoint) = &routes_output.rivetkit_endpoint {
			println!("  Endpoint:        https://{endpoint}");
		}
	}
	println!("  Dashboard:       {hub_origin}/projects/{project_slug}/environments/{env_slug}");
	if config.rivetkit.is_some() {
		println!("  Next Steps:      https://rivet.gg/docs/quickstart/actors");
	} else {
		println!("  Next Steps:      https://rivet.gg/docs/quickstart");
	}
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
	let default_route_subpaths = function.route_subpaths.unwrap_or(true);
	let default_strip_prefix = function.strip_prefix.unwrap_or(true);

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
	apis::routes_api::routes_update(
		&ctx.openapi_config_cloud,
		&fn_name,
		update_route_body.clone(),
		Some(&ctx.project.name_id.to_string()),
		Some(&environment.slug),
	)
	.await?;

	println!(
		"Successfully created route: {}{}",
		update_route_body.hostname, update_route_body.path
	);

	Ok(())
}

fn generate_actor_script(registry_path: String) -> String {
	formatdoc!(
		r#"
		// DO NOT EDIT
		//
		// This file is auto-generated by the Rivet CLI. This file should automatically be
		// removed once the deploy is complete.

		import {{ createActorHandler }} from "@rivetkit/actor/drivers/rivet";
		import {{ registry }} from "{registry_path}";

		export default createActorHandler(registry);
		"#,
	)
}

fn generate_server_dockerfile(project_root: &std::path::Path, server_path: String) -> String {
	// Detect package manager
	let package_manager = detect_package_manager(project_root);
	println!(
		"[RivetKit] Detected package manager: {}",
		package_manager.name
	);

	// Strip TypeScript extensions - Node.js will automatically resolve to the correct .js extension
	let server_path_js = if server_path.ends_with(".ts") {
		server_path[..server_path.len() - 3].to_string()
	} else if server_path.ends_with(".tsx") {
		server_path[..server_path.len() - 4].to_string()
	} else if server_path.ends_with(".mts") {
		server_path[..server_path.len() - 4].to_string()
	} else if server_path.ends_with(".cts") {
		server_path[..server_path.len() - 4].to_string()
	} else {
		server_path
	};

	generate_dockerfile_for_package_manager(&package_manager, &server_path_js)
}

fn generate_dockerfile_for_package_manager(
	package_manager: &PackageManager,
	server_path_js: &str,
) -> String {
	let copy_files = package_manager.copy_files.join(" ");

	// Determine package manager specific configurations
	let (base_image, setup_commands, build_cmd, runtime_cmd, add_deps_cmd) = match package_manager.name.as_str() {
		"yarn" => (
			"node:22-alpine",
			"# Install Yarn if not present\n\t\t\tRUN if ! command -v yarn &> /dev/null; then \\\n\t\t\t\techo \"[RivetKit] Installing Yarn\"; \\\n\t\t\t\tcorepack enable && corepack prepare yarn@stable --activate; \\\n\t\t\tfi",
			"yarn dlx tsc --outDir dist/ --rootDir ./",
			"node",
			"yarn add @hono/node-server @hono/node-ws"
		),
		"bun" => (
			"oven/bun:1-alpine",
			"",
			"bunx tsc --outDir dist/ --rootDir ./",
			"bun run",
			"bun add @hono/node-server @hono/node-ws"
		),
		"pnpm" => (
			"node:22-alpine",
			"# Install pnpm\n\t\t\tRUN corepack enable && corepack prepare pnpm@latest --activate",
			"pnpm dlx tsc --outDir dist/ --rootDir ./",
			"node",
			"pnpm add @hono/node-server @hono/node-ws"
		),
		_ => (
			"node:22-alpine",
			"",
			"npx tsc --outDir dist/ --rootDir ./",
			"node",
			"npm install @hono/node-server @hono/node-ws"
		)
	};

	let mut dockerfile = String::new();

	// Builder stage
	dockerfile.push_str(&format!("FROM {} AS builder\n\n", base_image));
	dockerfile.push_str("WORKDIR /app\n\n");
	dockerfile.push_str(&format!("# Copy package files\nCOPY {} ./\n\n", copy_files));

	// Setup package manager if needed
	if !setup_commands.is_empty() {
		dockerfile.push_str(setup_commands);
		dockerfile.push_str("\n\n");
	}

	// Install dependencies
	if package_manager.name == "npm" {
		dockerfile.push_str(&format!(
			"RUN {} \\\n\tif [ -f package-lock.json ]; then \\\n\t\techo \"[RivetKit] Installing dependencies with npm ci (lockfile found)\"; \\\n\t\t{}; \\\n\telse \\\n\t\techo \"[RivetKit] Installing dependencies with npm install (no lockfile)\"; \\\n\t\tnpm install; \\\n\tfi\n\n",
			package_manager.cache_mount,
			package_manager.install_cmd
		));
	} else {
		dockerfile.push_str(&format!(
			"RUN {} \\\n\techo \"[RivetKit] Installing dependencies with {}\"; \\\n\t{}\n\n",
			package_manager.cache_mount, package_manager.name, package_manager.install_cmd
		));
	}

	dockerfile.push_str("COPY . .\n\n");
	dockerfile.push_str(&format!("RUN {}\n\n", build_cmd));
	dockerfile.push_str("# ===\n\n");

	// Runtime stage
	dockerfile.push_str(&format!("FROM {} AS runtime\n\n", base_image));
	dockerfile
		.push_str("RUN addgroup -g 1001 -S rivet && \\\n\tadduser -S rivet -u 1001 -G rivet\n\n");
	dockerfile.push_str("WORKDIR /app\n\n");

	// Setup package manager in runtime if needed
	if !setup_commands.is_empty() {
		dockerfile.push_str(setup_commands);
		dockerfile.push_str("\n\n");
	}

	dockerfile.push_str(&format!(
		"COPY --from=builder --chown=rivet:rivet /app/{} ./\n\n",
		copy_files
	));

	// Install production dependencies
	if package_manager.name == "npm" {
		dockerfile.push_str(&format!(
			"RUN {} \\\n\tif [ -f package-lock.json ]; then \\\n\t\techo \"[RivetKit] Installing prod deps with npm ci\"; \\\n\t\t{}; \\\n\telse \\\n\t\techo \"[RivetKit] Installing prod deps with npm install\"; \\\n\t\tnpm install --production; \\\n\tfi && \\\n\techo \"[RivetKit] Adding Hono runtime deps\" && \\\n\t{}\n\n",
			package_manager.cache_mount,
			package_manager.install_prod_cmd,
			add_deps_cmd
		));
	} else {
		dockerfile.push_str(&format!(
			"RUN {} \\\n\techo \"[RivetKit] Installing production dependencies with {}\"; \\\n\t{} && \\\n\techo \"[RivetKit] Adding Hono runtime deps\" && \\\n\t{}\n\n",
			package_manager.cache_mount,
			package_manager.name,
			package_manager.install_prod_cmd,
			add_deps_cmd
		));
	}

	dockerfile.push_str("COPY --from=builder --chown=rivet:rivet /app/dist ./dist\n\n");
	dockerfile.push_str("USER rivet\n\n");

	// Final command
	if runtime_cmd == "bun run" {
		dockerfile.push_str(&format!(
			"CMD [\"bun\", \"run\", \"dist/{}\"]\n",
			server_path_js
		));
	} else {
		dockerfile.push_str(&format!("CMD [\"node\", \"dist/{}\"]\n", server_path_js));
	}

	dockerfile
}

fn generate_server_dockerignore() -> String {
	formatdoc!(
		r#"
		# Dependencies
		node_modules/
		npm-debug.log*
		yarn-debug.log*
		yarn-error.log*
		pnpm-debug.log*
		.pnpm-debug.log*
		bun-debug.log*
		.bun-debug.log*

		# Build outputs
		dist/
		build/
		.next/
		.nuxt/

		# Development files
		.env*
		.vscode/
		.idea/
		*.log
		.DS_Store
		Thumbs.db

		# Git
		.git/
		.gitignore
		README.md
		
		# Testing
		coverage/
		.nyc_output/
		test/
		tests/
		__tests__/
		*.test.*
		*.spec.*

		# TypeScript
		*.tsbuildinfo
		"#
	)
}
