use anyhow::*;
use std::{collections::HashMap, path::Path};

use crate::{
	config::{self},
	toolchain_ctx::ToolchainCtx,
	util::{
		cmd::{self, shell_cmd},
		task,
	},
};

pub struct BuildImageOutput {
	pub tag: String,
	pub path: tempfile::TempPath,
}

/// Builds an image and archives it to a path.
pub async fn build_image(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	build_path: &Path,
	dockerfile: &Path,
	build_method: config::build::docker::BuildMethod,
	build_kind: config::build::docker::BundleKind,
	build_compression: config::build::Compression,
	build_args: Option<&[String]>,
	build_target: Option<&str>,
	allow_root: bool,
) -> Result<BuildImageOutput> {
	// Determine build method
	let build_method = if build_method == config::build::docker::BuildMethod::Buildx {
		// Validate that Buildx is installed
		let mut buildx_version_cmd = shell_cmd("docker");
		buildx_version_cmd.args(&["buildx", "version"]);
		let buildx_version = cmd::execute_docker_cmd_silent_fallible(buildx_version_cmd).await?;

		if buildx_version.status.success() {
			config::build::docker::BuildMethod::Buildx
		} else {
			task.log("Docker Buildx not installed. Falling back to native build method.\n\nPlease install Buildx here: https://github.com/docker/buildx#installing");
			config::build::docker::BuildMethod::Native
		}
	} else {
		build_method
	};

	let buildx_info = match build_method {
		config::build::docker::BuildMethod::Native => " (with native)",
		config::build::docker::BuildMethod::Buildx => " (with buildx)",
	};
	task.log(format!("[Building] {}{buildx_info}", dockerfile.display()));

	// Build args
	let mut build_arg_flags = HashMap::<String, String>::new();
	if let Some(build_args) = build_args {
		for item in build_args {
			let (k, v) = item
				.split_once('=')
				.context("Build arg missing '=': {item}")?;
			ensure!(
				!k.starts_with("RIVET_"),
				"Build arg must not start with 'RIVET_': {k}"
			);
			build_arg_flags.insert(k.into(), v.into());
		}
	}
	build_arg_flags.insert("RIVET_API_ENDPOINT".into(), ctx.api_endpoint.clone());

	// Build image
	let image_tag = super::generate_unique_image_tag();
	match build_method {
		config::build::docker::BuildMethod::Native => {
			let mut build_cmd = shell_cmd("docker");
			build_cmd
				.arg("build")
				.arg("--platform")
				.arg("linux/amd64")
				.arg("--file")
				.arg(dockerfile)
				.arg("--tag")
				.arg(&image_tag)
				.args(
					&build_arg_flags
						.iter()
						.map(|(k, v)| format!("--build-arg={}={}", k, v))
						.collect::<Vec<String>>(),
				);
			if let Some(build_target) = build_target {
				build_cmd.arg("--target").arg(build_target);
			}
			build_cmd.arg(build_path);
			cmd::execute_docker_cmd(
				task.clone(),
				build_cmd,
				"Docker image failed to build (native)",
			)
			.await?;
		}
		config::build::docker::BuildMethod::Buildx => {
			let builder_name = "rivet_toolchain";

			// Determine if needs to create a new builder
			let mut inspect_cmd = shell_cmd("docker");
			inspect_cmd.arg("buildx").arg("inspect").arg(builder_name);
			let inspect_output = cmd::execute_docker_cmd_silent_fallible(inspect_cmd).await?;

			if !inspect_output.status.success()
				&& String::from_utf8(inspect_output.stderr.clone())?
					.contains(&format!("no builder \"{builder_name}\" found"))
			{
				// Create new builder

				let mut build_cmd = shell_cmd("docker");
				build_cmd
					.arg("buildx")
					.arg("create")
					.arg("--name")
					.arg(builder_name)
					.arg("--driver")
					.arg("docker-container")
					.arg("--platform")
					.arg("linux/amd64");
				cmd::execute_docker_cmd(
					task.clone(),
					build_cmd,
					"Failed to create Docker Buildx builder",
				)
				.await?;
			} else {
				// Builder exists

				cmd::error_for_output_failure(
					&inspect_output,
					"Failed to inspect Docker Buildx runner",
				)?;
			}

			// Build image
			let mut build_cmd = shell_cmd("docker");
			build_cmd
				.arg("buildx")
				.arg("build")
				.arg("--builder")
				.arg(builder_name)
				.arg("--platform")
				.arg("linux/amd64")
				.arg("--file")
				.arg(dockerfile)
				.arg("--tag")
				.arg(&image_tag)
				.args(
					&build_arg_flags
						.iter()
						.map(|(k, v)| format!("--build-arg={}={}", k, v))
						.collect::<Vec<String>>(),
				)
				.arg("--output")
				.arg("type=docker");
			if let Some(build_target) = build_target {
				build_cmd.arg("--target").arg(build_target);
			}
			build_cmd.arg(build_path);
			cmd::execute_docker_cmd(
				task.clone(),
				build_cmd,
				"Docker image failed to build (buildx)",
			)
			.await?;
		}
	}

	// Build archive
	let build_tar_path = super::archive::create_archive(
		task.clone(),
		&image_tag,
		build_kind,
		build_compression,
		allow_root,
	)
	.await?;

	// Clean up image from the registry
	let mut remove_img_cmd = shell_cmd("docker");
	remove_img_cmd
		.arg("image")
		.arg("rm")
		.arg("--force")
		.arg(&image_tag);
	cmd::execute_docker_cmd_silent_fallible(remove_img_cmd).await?;

	Ok(BuildImageOutput {
		tag: image_tag,
		path: build_tar_path,
	})
}
