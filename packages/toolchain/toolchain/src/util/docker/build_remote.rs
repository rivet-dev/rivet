use anyhow::*;
use flate2::{write::GzEncoder, Compression};
use serde_json::json;
use std::{collections::HashMap, io::Write, path::{Path, PathBuf}, result::Result::Ok, time::Duration};
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::{
	config::{self},
	project::environment::TEMPEnvironment,
	toolchain_ctx::ToolchainCtx,
	util::{docker::push, task},
};

use rivet_api::{apis, models};

const DEFAULT_CI_REGION: &str = "atl";

/// Environment slug for the CI environment.
const CI_ENVIRONMENT_ID: &str = "ci";

/// Release URLs for CI components
///
/// Uploaded with ./scripts/cloud/upload-builds.ts
const CI_MANAGER_RELEASE_URL: &str =
	"https://releases.rivet.gg/ci-manager/2025-06-10-09-25-53-481Z/image.tar";
const CI_RUNNER_RELEASE_URL: &str =
	"https://releases.rivet.gg/ci-runner/2025-06-10-08-48-55-332Z/image.tar";

pub async fn build_remote(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	environment: TEMPEnvironment,
	build_path: &Path,
	dockerfile: &Path,
	build_arg_flags: &Option<Vec<String>>,
	build_target: &Option<String>,
) -> Result<push::PushOutput> {
	task.log("[Remote Build] Starting remote build process");

	// Get or create CI namespace
	let ci_env = get_or_create_ci_namespace(ctx, task.clone()).await?;

	// Get build IDs for CI manager and runner
	let ci_manager_build_id = upload_ci_manager_build(ctx, task.clone(), &ci_env).await?;
	let ci_runner_build_id = upload_ci_runner_build(ctx, task.clone(), &ci_env).await?;

	// Get or create ci-manager actor
	let (ci_runner_actor_id, ci_runner_endpoint) =
		get_or_create_ci_manager_actor(ctx, task.clone(), ci_manager_build_id, ci_runner_build_id)
			.await?;

	// Upload build context
	let build_id = upload_build_context(
		ctx,
		task.clone(),
		&environment,
		build_path,
		dockerfile,
		build_arg_flags,
		build_target,
		&ci_runner_actor_id,
		&ci_runner_endpoint,
	)
	.await?;

	// Poll build status
	let build_id = poll_build_status(ctx, task.clone(), &build_id, &ci_runner_endpoint).await?;

	Ok(push::PushOutput { build_id })
}

async fn get_or_create_ci_namespace(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
) -> Result<TEMPEnvironment> {
	// Look for existing CI namespace
	let existing_namespace = ctx
		.project
		.namespaces
		.iter()
		.find(|ns| ns.name_id == CI_ENVIRONMENT_ID);

	if let Some(namespace) = existing_namespace {
		task.log(format!(
			"[Remote Build] Found existing CI namespace: {}",
			namespace.name_id
		));
		return Ok(TEMPEnvironment::from(namespace.clone()));
	}

	// Create new CI namespace using the default version
	task.log("[Remote Build] Creating new CI namespace");

	// Get the default version (first version if available)
	let default_version_id = ctx
		.project
		.versions
		.first()
		.map(|v| v.version_id)
		.context("No versions available for project")?;

	let create_response =
		apis::cloud_games_namespaces_api::cloud_games_namespaces_create_game_namespace(
			&ctx.openapi_config_cloud,
			&ctx.project.game_id.to_string(),
			models::CloudGamesNamespacesCreateGameNamespaceRequest {
				name_id: CI_ENVIRONMENT_ID.to_string(),
				display_name: "Continuous Integration".to_string(),
				version_id: default_version_id,
			},
		)
		.await
		.context("Failed to create CI namespace")?;

	// Fetch the full namespace details
	let namespace_response =
		apis::cloud_games_namespaces_api::cloud_games_namespaces_get_game_namespace_by_id(
			&ctx.openapi_config_cloud,
			&ctx.project.game_id.to_string(),
			&create_response.namespace_id.to_string(),
		)
		.await
		.context("Failed to get created CI namespace details")?;

	Ok(TEMPEnvironment::from(*namespace_response.namespace))
}

async fn upload_ci_manager_build(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	ci_env: &TEMPEnvironment
) -> Result<Uuid> {
	upload_ci_build(ctx, task, ci_env, "ci-manager", CI_MANAGER_RELEASE_URL).await
}

async fn upload_ci_runner_build(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	ci_env: &TEMPEnvironment
) -> Result<Uuid> {
	upload_ci_build(ctx, task, ci_env, "ci-runner", CI_RUNNER_RELEASE_URL).await
}

async fn upload_ci_build(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	ci_env: &TEMPEnvironment,
	name: &str,
	url: &str,
) -> Result<Uuid> {
	task.log(format!(
		"[Remote Build] Checking for existing {} build",
		name
	));

	// Check if build already exists with this URL tag
	let tags_filter = serde_json::to_string(&serde_json::json!({
		"name": name,
		"url": url
	}))?;

	let builds_response = apis::builds_api::builds_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(&ci_env.slug),
		Some(&tags_filter),
	)
	.await
	.context("Failed to list builds")?;

	// If build exists, return its ID
	if let Some(existing_build) = builds_response.builds.first() {
		task.log(format!(
			"[Remote Build] Found existing {} build: {}",
			name, existing_build.id
		));
		return Ok(existing_build.id);
	}

	// Download the image.tar
	task.log(format!("[Remote Build] Downloading {} from {}", name, url));
	let temp_file = download_file(url)
		.await
		.context("Failed to download image.tar")?;

	// Upload the build using push_tar
	task.log(format!("[Remote Build] Uploading {} build", name));
	let push_opts = push::PushOpts {
		env: ci_env.clone(),
		path: temp_file.path().to_path_buf(),
		docker_tag: format!("{}:{}", name, Uuid::new_v4()),
		bundle: config::build::docker::BundleKind::DockerImage,
		compression: config::build::Compression::None,
	};

	let push_output = push::push_tar(ctx, task.clone(), &push_opts)
		.await
		.context("Failed to upload build")?;

	// Patch tags on the build
	task.log(format!("[Remote Build] Tagging {} build", name));
	let patch_tags = serde_json::json!({
		"name": name,
		"url": url
	});

	apis::builds_api::builds_patch_tags(
		&ctx.openapi_config_cloud,
		&push_output.build_id.to_string(),
		models::BuildsPatchBuildTagsRequest {
			tags: Some(patch_tags),
			exclusive_tags: None,
		},
		Some(&ctx.project.name_id),
		Some(&ci_env.slug),
	)
	.await
	.context("Failed to patch build tags")?;

	task.log(format!(
		"[Remote Build] {} build uploaded successfully: {}",
		name, push_output.build_id
	));
	Ok(push_output.build_id)
}

async fn download_file(url: &str) -> Result<NamedTempFile> {
	let response = reqwest::get(url)
		.await
		.context("Failed to fetch file")?
		.error_for_status()
		.context("HTTP error while fetching file")?;

	let mut temp_file = NamedTempFile::new().context("Failed to create temporary file")?;

	let bytes = response
		.bytes()
		.await
		.context("Failed to read response body")?;

	temp_file
		.write_all(&bytes)
		.context("Failed to write to temporary file")?;

	temp_file
		.flush()
		.context("Failed to flush temporary file")?;

	Ok(temp_file)
}

async fn get_or_create_ci_manager_actor(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	ci_manager_build_id: Uuid,
	ci_runner_build_id: Uuid,
) -> Result<(String, String)> {
	// Check if default region exists
	let regions_response = apis::regions_api::regions_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(CI_ENVIRONMENT_ID),
	)
	.await
	.context("Failed to list regions")?;

	let default_region_exists = regions_response
		.regions
		.iter()
		.any(|region| region.id == DEFAULT_CI_REGION);

	if default_region_exists {
		task.log(format!("[Remote Build] {DEFAULT_CI_REGION} region found"));
	} else {
		task.log(format!(
			"[Remote Build] {DEFAULT_CI_REGION} region not found, will not specify region"
		));
	}

	// Look for existing ci-manager actor
	let tags_json = serde_json::to_string(&serde_json::json!({"name": "ci-manager"}))?;
	let actors_response = apis::actors_api::actors_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(CI_ENVIRONMENT_ID),
		None,
		Some(&tags_json),
		Some(false),
		None,
	)
	.await
	.context("Failed to list actors")?;

	if let Some(existing_actor) = actors_response.actors.iter().find(|actor| {
		actor
			.tags
			.as_ref()
			.and_then(|tags| tags.get("name"))
			.and_then(|v| v.as_str())
			== Some("ci-manager")
	}) {
		task.log(format!(
			"[Remote Build] Found existing ci-manager actor: {}",
			existing_actor.id
		));

		let endpoint = existing_actor
			.network
			.ports
			.get("http")
			.and_then(|port| port.hostname.as_ref())
			.context("CI manager actor endpoint not available")?;

		return Ok((existing_actor.id.to_string(), endpoint.clone()));
	}

	// Create new ci-manager actor
	task.log("[Remote Build] Creating new ci-manager actor");
	let create_response = apis::actors_api::actors_create(
		&ctx.openapi_config_cloud,
		models::ActorsCreateActorRequest {
			region: if default_region_exists {
				Some(DEFAULT_CI_REGION.to_string())
			} else {
				None
			},
			tags: Some(json!({"name": "ci-manager"})),
			build: Some(ci_manager_build_id.into()),
			build_tags: None,
			runtime: Some(Box::new(models::ActorsCreateActorRuntimeRequest {
				environment: Some(HashMap::from([
					("KANIKO_EXECUTION_MODE".into(), "rivet".into()),
					("KANIKO_BUILD_ID".into(), ci_runner_build_id.to_string()),
					("RIVET_CLOUD_TOKEN".into(), ctx.access_token.clone()),
					("RIVET_PROJECT".into(), ctx.project.name_id.clone()),
					("RIVET_ENVIRONMENT".into(), CI_ENVIRONMENT_ID.into()),
				])),
				network: None,
			})),
			network: Some(Box::new(models::ActorsCreateActorNetworkRequest {
				mode: Some(models::ActorsNetworkMode::Bridge),
				ports: Some(std::collections::HashMap::from([(
					"http".to_string(),
					models::ActorsCreateActorPortRequest {
						protocol: models::ActorsPortProtocol::Https,
						routing: Some(Box::new(models::ActorsPortRouting {
							guard: Some(serde_json::json!({})),
							host: None,
						})),
						internal_port: Some(3000),
					},
				)])),
				wait_ready: Some(true),
			})),
			resources: Some(Box::new(models::ActorsResources {
				cpu: 1000,
				memory: 1024,
			})),
			lifecycle: Some(Box::new(models::ActorsLifecycle {
				kill_timeout: Some(30000),
				durable: Some(true),
			})),
		},
		Some(&ctx.project.name_id),
		Some(CI_ENVIRONMENT_ID),
		None,
	)
	.await
	.context("Failed to create ci-manager actor")?;

	let endpoint = create_response
		.actor
		.network
		.ports
		.get("http")
		.and_then(|port| port.hostname.as_ref())
		.context("CI manager actor endpoint not available")?;

	Ok((create_response.actor.id.to_string(), endpoint.clone()))
}

/// Moves over build context to a temp directory,
/// ignoring all .dockerignore files.
fn prepare_build_context_dir(
	build_path: &Path,
	dockerfile_path: &Path
) -> Result<Vec<PathBuf>> {
	const DOCKERIGNORE_FILENAME: &str = ".dockerignore";

	let mut paths = Vec::new();

	// For Kaniko on the remote build, we need to expose
	// our Dockerfile and our .dockerignore file if it
	// exists
	paths.push(dockerfile_path.to_path_buf());

	let dockerignore_path = build_path.join(DOCKERIGNORE_FILENAME);
	if dockerignore_path.try_exists().unwrap_or(false) {
		paths.push(dockerignore_path.clone());
	}

	let walk = ignore::WalkBuilder::new(build_path)
		.standard_filters(false)
		.add_custom_ignore_filename(DOCKERIGNORE_FILENAME)
		.parents(true)
		.build();
		
	for entry in walk {
		let entry = entry?;

		if entry.path() == dockerfile_path  || entry.path() == &dockerignore_path {
			// Skip the Dockerfile or .dockerignore itself, we already added it
			continue;
		}

		let is_file = entry.file_type()
			.map(|ft| ft.is_file())
			.unwrap_or(false);

		if is_file {
			let file_path = entry.path();

			paths.push(file_path.to_path_buf());
		}
	}

	Ok(paths)
}

async fn create_build_context_archive(
	task: task::TaskCtx,
	build_path: &Path,
	dockerfile: &Path,
) -> Result<Vec<u8>> {
	task.log(format!(
		"[Remote Build] Creating gzipped tar archive from build path: {:?}",
		build_path
	));

	let dockerfile_path_buf = build_path.join(dockerfile);
	let dockerfile = Path::new(&dockerfile_path_buf);

	// Get a list of all paths that weren't in .dockerignore
	let build_file_paths = prepare_build_context_dir(build_path, dockerfile)?;

	// Create a gzipped tar archive of the build context
	let mut archive_data = Vec::new();
	{
		let gz_encoder = GzEncoder::new(&mut archive_data, Compression::default());
		let mut tar = tar::Builder::new(gz_encoder);

		// Add the prepared build file paths to the archive
		for file_path in build_file_paths.iter() {
			let relative_path = file_path.strip_prefix(build_path)
				.context("Failed to strip build path prefix")?;

			tar.append_path_with_name(&file_path, relative_path)
				.context(format!("Failed to add file to tar: {:?}", file_path))?;
		}

		tar.finish().context("Failed to finalize tar archive")?;
	}

	task.log(format!(
		"[Remote Build] Created gzipped archive ({} bytes)",
		archive_data.len()
	));
	Ok(archive_data)
}

async fn upload_build_context(
	_ctx: &ToolchainCtx,
	task: task::TaskCtx,
	environment: &TEMPEnvironment,
	build_path: &Path,
	dockerfile: &Path,
	build_arg_flags: &Option<Vec<String>>,
	build_target: &Option<String>,
	_ci_manager_actor_id: &str,
	ci_manager_endpoint: &str,
) -> Result<String> {
	let server_url = format!("https://{}", ci_manager_endpoint);
	task.log(format!(
		"[Remote Build] Using CI manager endpoint: {}",
		server_url
	));

	// Create build context tar.gz
	let context_buffer = create_build_context_archive(task.clone(), build_path, dockerfile).await?;
	let build_name = "rivet-remote-build";

	// Prepare multipart form data
	task.log("[Remote Build] Uploading build context...");

	// Serialize build args if provided
	let serialized_build_args = serde_json::to_string(
		build_arg_flags.as_deref().unwrap_or(&[])
	).context("Failed to serialize build args")?;

	// Create FormData-like structure using reqwest
	let form = reqwest::multipart::Form::new()
		.text("buildName", build_name)
		.text("environmentId", environment.slug.to_string())
		.text("buildArgs", serialized_build_args)
		.text(
			"dockerfilePath",
			dockerfile
				.file_name()
				.and_then(|name| name.to_str())
				.unwrap_or("Dockerfile")
				.to_string(),
		)
		.part(
			"context",
			reqwest::multipart::Part::bytes(context_buffer)
				.file_name("context.tar.gz")
				.mime_str("application/gzip")?,
		);
	
	let form = if let Some(target) = build_target {
		form.text("buildTarget", target.clone())
	} else {
		form
	};

	// Submit build
	let encoded_server_url = server_url.replace(":", "%3A").replace("/", "%2F");
	let response = reqwest::Client::new()
		.post(&format!(
			"{}/builds?serverUrl={}",
			server_url, encoded_server_url
		))
		.multipart(form)
		.send()
		.await
		.context("Failed to upload build context")?;

	if !response.status().is_success() {
		let status = response.status();
		let error_text = response
			.text()
			.await
			.unwrap_or_else(|_| "Unknown error".to_string());
		bail!("Build upload failed with status {}: {}", status, error_text);
	}

	let result: serde_json::Value = response
		.json()
		.await
		.context("Failed to parse build upload response")?;

	let build_id = result
		.get("buildId")
		.and_then(|v| v.as_str())
		.context("Build response missing buildId")?
		.to_string();

	task.log(format!(
		"[Remote Build] Build context uploaded successfully, build ID: {}",
		build_id
	));
	Ok(build_id)
}

async fn poll_build_status(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	build_id: &str,
	ci_manager_endpoint: &str,
) -> Result<Uuid> {
	let server_url = format!("https://{}", ci_manager_endpoint);

	// Poll build status until completion
	task.log("[Remote Build] Polling build status...");
	let max_timeout = Duration::from_secs(30 * 60);
	let poll_interval = Duration::from_secs(2);
	let start_time = std::time::Instant::now();
	let mut previous_status = String::new();
	loop {
		if start_time.elapsed() >= max_timeout {
			bail!("Build polling timeout after 30 minutes");
		}

		tokio::time::sleep(poll_interval).await;

		let response = reqwest::Client::new()
			.get(&format!("{}/builds/{}", server_url, build_id))
			.send()
			.await;

		match response {
			Ok(res) => {
				if res.status().is_success() {
					let build_info: serde_json::Value = res
						.json()
						.await
						.context("Failed to parse build status response")?;

					let status = build_info
						.get("status")
						.and_then(|s| s.get("type"))
						.and_then(|t| t.as_str())
						.unwrap_or("unknown");

					let is_status_changed = previous_status != status;
					if is_status_changed {
						previous_status = status.to_string();
					}

					match status {
						"success" => {
							task.log("[Remote Build] Build completed successfully");
							let build_id_raw = build_info
								.get("status")
								.and_then(|x| x.get("data"))
								.and_then(|x| x.get("buildId"))
								.and_then(|x| x.as_str())
								.context("missing build id in status")?;
							let build_id = Uuid::parse_str(&build_id_raw)?;
							return Ok(build_id);
						}
						"failure" => {
							let reason = build_info
								.get("status")
								.and_then(|s| s.get("data"))
								.and_then(|d| d.get("reason"))
								.and_then(|r| r.as_str())
								.unwrap_or("Unknown error");
							bail!("Remote build failed: {}", reason);
						}
						"running" => {
							task.log(format!("[Remote Build] Build status: {}", status));

							// If we know its running, and have its actor ID,
							// we can stop polling and start streaming logs
							// until the actor exists.
							let runner = build_info
								.get("status")
								.and_then(|s| s.get("data"))
								.and_then(|d| d.get("rivet"));
							if let Some(runner) = runner {
								let actor_id_raw = runner
									.get("actorId")
									.and_then(|a| a.as_str())
									.context("Rivet runner missing actorId")?;

								let actor_id = Uuid::parse_str(&actor_id_raw)
									.context("Failed to parse actor ID from rivet runner")?;

								task.log("[Remote Build] Streaming build logs from actor.");

								match crate::util::actor::logs::tail(
									&ctx,
									crate::util::actor::logs::TailOpts {
										environment: CI_ENVIRONMENT_ID,
										actor_id,
										stream: crate::util::actor::logs::LogStream::All,
										follow: true,
										print_type: crate::util::actor::logs::PrintType::Custom(handle_build_log_line),
										exit_on_ctrl_c: false
									},
								)
								.await {
									Ok(_) => {
										task.log("[Remote Build] Build logs streaming completed.");
									}
									Err(e) => {
										task.log(format!(
											"[Remote Build] Failed to stream build logs: {}",
											e
										));
									}
								}
							}
						}
						_ => {
							if is_status_changed {
								task.log(format!("[Remote Build] Build status: {}", status));
							}
							// Continue polling for other statuses (pending, running, etc.)
						}
					}
				} else {
					task.log(format!(
						"[Remote Build] Poll failed: HTTP {}",
						res.status()
					));
				}
			}
			Err(e) => {
				task.log(format!(
					"[Remote Build] Poll failed: {}",
					e
				));
			}
		}
	}
}

fn handle_build_log_line(
	_timestamp: chrono::DateTime<chrono::Utc>,
	line: String,
) {
	let line = strip_ansi_escape_codes(&line);

	// If the line starts with INFO[.+], its a Kaniko log line
	// so we strip the prefix and only print it if its important
	let Some(line) = transform_log_line(line) else {
		return;
	};

	println!("[Remote Build] {line}");
}

fn transform_log_line(line: String) -> Option<String> {
	let Some(stripped) = line.strip_prefix("INFO[") else {
		return Some(format!("> {}", line).to_string());
	};

	let end_index = stripped.find(']').unwrap_or(stripped.len());
	let stripped = &stripped[end_index + 1..].trim();

	let line: String = stripped.to_string();

	// If it starts with uppercase word, its probably important
	// since it's probably a Dockerfile instruction
	let first_word = &line.split_whitespace()
		.next()
		.unwrap_or("");
	let is_docker_instruction = first_word
		.chars()
		.filter(|c| c.is_alphabetic())
		.all(|c| c.is_uppercase());

	if is_docker_instruction {
		return Some(line);
	}
	
	if line.starts_with("Unpacking rootfs") {
		return Some("Initializing image filesystem...".to_string());
	}

	if line.starts_with("Taking snapshot of full filesystem") {
		return Some("Taking snapshot of filesystem...".to_string());
	}

	if line.starts_with("Uploading tar file") {
		return Some("Exporting built image...".to_string());
	}

	None
}

fn strip_ansi_escape_codes(line: &str) -> String {
	// If the input doesn't contain escape sequences, don't process it,
	// as strip_ansi_escapes happens to strip tabs as well.
	// (See https://github.com/luser/strip-ansi-escapes/issues/20)
	if line.contains('\x1b') {
		return strip_ansi_escapes::strip_str(line).to_string()
	}

	line.to_string()
}

async fn _get_build_by_tags(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	environment: &TEMPEnvironment,
) -> Result<Uuid> {
	// List builds in the project to find one matching our build
	task.log("[Remote Build] Listing builds to find final build...");

	// Create tags filter for the remote build
	let tags_filter = serde_json::to_string(&serde_json::json!({
		"name": "rivet-remote-build"
	}))?;

	let builds_response = apis::builds_api::builds_list(
		&ctx.openapi_config_cloud,
		Some(&ctx.project.name_id),
		Some(&environment.slug),
		Some(&tags_filter),
	)
	.await
	.context("Failed to list builds")?;

	// Get the first matching build
	if let Some(matching_build) = builds_response.builds.iter().next() {
		task.log(format!(
			"[Remote Build] Found matching build: {}",
			matching_build.id
		));
		Ok(matching_build.id)
	} else {
		bail!("No matching build found");
	}
}
