use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use rivet_api::{apis, models};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::fs;
use uuid::Uuid;

use crate::{
	config, paths,
	project::environment::TEMPEnvironment,
	toolchain_ctx::ToolchainCtx,
	util::{js_utils, net::upload, task, term},
};

/// File name for the index path to the script.
const BUILD_INDEX_NAME: &str = "index.js";

pub struct BuildAndUploadOpts {
	pub env: TEMPEnvironment,
	pub tags: HashMap<String, String>,
	pub build_config: config::build::javascript::Build,
}

/// Builds image if not specified and returns the build ID.
pub async fn build_and_upload(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	opts: BuildAndUploadOpts,
) -> Result<Uuid> {
	task.log(format!("[Building] {}", kv_str::to_str(&opts.tags)?));

	let project_root = paths::project_root()?;

	// Create dir to write build artifacts to
	let build_dir = tempfile::TempDir::new()?;

	if opts.build_config.unstable.dump_build() {
		task.log(format!("[Build Path] {}", build_dir.path().display()));
	}

	// Bundle JS
	if !opts.build_config.unstable.no_bundler() {
		// Validate that the script path has a .ts or .js extension
		let script_path = project_root.join(&opts.build_config.script);
		let ext = script_path.extension().and_then(|s| s.to_str());
		ensure!(
			ext == Some("ts") || ext == Some("tsx") || ext == Some("js") || ext == Some("jsx"),
			"script file must have a .ts or .js extension for Deno bundler"
		);

		// Check the project before deploying
		//deno_check_script(CheckOpts {
		//	script_path: &script_path,
		//	config_path: config_path.as_deref(),
		//	import_map_url: import_map_url.as_deref(),
		//	lock_path: lock_path.as_deref(),
		//})
		//.await?;

		// Build the bundle to the output dir. This will bundle all Deno dependencies into a
		// single JS file.
		//
		// The Deno command is run in the project root, so `config_path`, `lock_path`, etc can
		// all safely be passed as relative paths without joining with `project_root`.
		let output = js_utils::run_command_and_parse_output::<
			js_utils::schemas::build::Input,
			js_utils::schemas::build::Output,
		>(
			"src/tasks/build/mod.js",
			&js_utils::schemas::build::Input {
				entry_point: script_path,
				out_dir: build_dir.path().to_path_buf(),
				bundle: js_utils::schemas::build::Bundle {
					minify: opts.build_config.unstable.minify(),
					analyze_result: opts.build_config.unstable.analyze_result(),
					log_level: opts.build_config.unstable.esbuild_log_level(),
				},
			},
		)
		.await?;
		if let Some(analyze_result) = output.analyzed_metafile {
			task.log("[Bundle Analysis]");
			task.log(analyze_result);
		}
	} else {
		// Ensure the script path has a .js extension
		let script_path = project_root.join(opts.build_config.script);
		ensure!(
			script_path.extension().and_then(|s| s.to_str()) == Some("js"),
			"script file must have a .js extension when not using a bundler"
		);

		// Validate script exists
		match fs::metadata(&script_path).await {
			Result::Ok(_) => {}
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
				bail!("script not found: {}", script_path.display())
			}
			Err(err) => bail!("failed to read script at {}: {err}", script_path.display()),
		}

		// Copy index file to build dir
		fs::copy(&script_path, build_dir.path().join(BUILD_INDEX_NAME)).await?;
	};

	// Deploy JS build
	let build_id = upload_bundle(
		ctx,
		task.clone(),
		&UploadBundleOpts {
			env: opts.env,
			build_path: build_dir.path().into(),
			compression: opts.build_config.unstable.compression(),
		},
	)
	.await?;

	// Retain build folder
	if opts.build_config.unstable.dump_build() {
		let _ = build_dir.into_path();
	}

	Ok(build_id)
}

// struct CheckOpts<'a> {
// 	script_path: &'a Path,
// 	config_path: Option<&'a str>,
// 	import_map_url: Option<&'a str>,
// 	lock_path: Option<&'a str>,
// }
//
// async fn deno_check_script(opts: CheckOpts<'_>) -> Result<()> {
// 	let deno_exec = deno_embed::get_executable(&paths::data_dir()?).await?;
//
// 	let mut check_cmd = Command::new(&deno_exec.executable_path);
// 	check_cmd.current_dir(&paths::project_root()?);
// 	check_cmd.env("DENO_NO_UPDATE_CHECK", "1");
// 	check_cmd.arg("check");
// 	if let Some(config_path) = &opts.config_path {
// 		check_cmd.arg(format!("--config={config_path}"));
// 	}
// 	if let Some(url) = &opts.import_map_url {
// 		check_cmd.arg(format!("--import-map={url}"));
// 	}
// 	if let Some(lock_path) = &opts.lock_path {
// 		check_cmd.arg(format!("--lock={lock_path}"));
// 	}
// 	check_cmd.arg(opts.script_path);
//
// 	let output = check_cmd
// 		.output()
// 		.await
// 		.map_err(|err| anyhow!("Failed to run command: {err}"))?;
//
// 	let stdout = String::from_utf8_lossy(&output.stdout);
// 	let stderr = String::from_utf8_lossy(&output.stderr);
//
// 	if output.status.success() {
// 		Ok(())
// 	} else {
// 		let mut error_message = format!("Failed to check script: {}", output.status);
// 		if !stdout.is_empty() {
// 			error_message.push_str(&format!("\nstdout:\n{stdout}"));
// 		}
// 		if !stderr.is_empty() {
// 			error_message.push_str(&format!("\nstderr:\n{stderr}"));
// 		}
// 		Err(anyhow!(error_message))
// 	}
// }

struct UploadBundleOpts {
	env: TEMPEnvironment,

	/// Path to the root of the built files.
	build_path: PathBuf,

	compression: config::build::Compression,
}

/// Uploads a built JavaScript bundle.
async fn upload_bundle(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	push_opts: &UploadBundleOpts,
) -> Result<Uuid> {
	// Validate bundle
	match fs::metadata(push_opts.build_path.join(BUILD_INDEX_NAME)).await {
		Result::Ok(_) => {}
		Err(err) => {
			if err.kind() == std::io::ErrorKind::NotFound {
				bail!("index.js does not exist in javascript bundle")
			} else {
				bail!("error reading javascript index.js: {err}")
			}
		}
	}

	// Archive build
	let build_tar_file = tempfile::NamedTempFile::new()?;
	let mut build_archive = tar::Builder::new(build_tar_file);
	build_archive.append_dir_all(".", &push_opts.build_path)?;
	let build_tar_file = build_archive.into_inner()?;

	let build_kind = models::ActorBuildKind::Javascript;
	let build_compression = match push_opts.compression {
		config::build::Compression::None => models::ActorBuildCompression::None,
		config::build::Compression::Lz4 => models::ActorBuildCompression::Lz4,
	};

	// Compress build
	let compressed_path =
		crate::util::build::compress_build(build_tar_file.as_ref(), push_opts.compression).await?;

	let image_file = upload::prepare_upload_file(
		&compressed_path,
		&crate::util::build::file_name(build_kind, build_compression),
		fs::metadata(&compressed_path).await?,
	)?;
	let files = vec![image_file.clone()];

	let total_len = files
		.iter()
		.fold(0, |acc, x| acc + x.prepared.content_length);

	task.log(format!(
		"[Uploading] {size}",
		size = upload::format_file_size(total_len as u64)?,
	));

	let prepare_res = apis::actor_builds_api::actor_builds_prepare(
		&ctx.openapi_config_cloud,
		models::ActorPrepareBuildRequest {
			image_tag: None,
			image_file: Box::new(image_file.prepared),
			kind: Some(build_kind),
			compression: Some(build_compression),
		},
		Some(&ctx.project.name_id),
		Some(&push_opts.env.slug),
	)
	.await
	.map_err(|err| anyhow!("Failed to prepare deploy: {err}"))?;

	// Upload files
	let reqwest_client = Arc::new(reqwest::Client::new());
	let pb = term::EitherProgressBar::Multi(term::multi_progress_bar(task.clone()));

	futures_util::stream::iter(prepare_res.presigned_requests)
		.map(Ok)
		.try_for_each_concurrent(8, |presigned_req| {
			let task = task.clone();
			let pb = pb.clone();
			let files = files.clone();
			let reqwest_client = reqwest_client.clone();

			async move {
				// Find the matching prepared file
				let file = files
					.iter()
					.find(|f| f.prepared.path == presigned_req.path)
					.context("missing prepared file")?;

				upload::upload_file(
					task.clone(),
					&reqwest_client,
					&presigned_req,
					&file.absolute_path,
					file.prepared.content_type.as_ref(),
					pb,
				)
				.await?;

				Result::<()>::Ok(())
			}
		})
		.await?;

	let complete_res = apis::actor_builds_api::actor_builds_complete(
		&ctx.openapi_config_cloud,
		&prepare_res.build.to_string(),
		Some(&ctx.project.name_id),
		Some(&push_opts.env.slug),
	)
	.await;
	if let Err(err) = complete_res.as_ref() {
		task.log(format!("{err:?}"));
	}
	complete_res.context("complete_res")?;

	Ok(prepare_res.build)
}
