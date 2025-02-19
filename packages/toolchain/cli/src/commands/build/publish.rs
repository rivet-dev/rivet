use anyhow::*;
use clap::Parser;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};
use toolchain::{
	config, errors,
	rivet_api::apis,
	tasks::{build_publish, get_bootstrap_data},
};

use crate::util::task::{run_task, TaskOutputStyle};

#[derive(Parser)]
pub struct Opts {
	#[clap(index = 1)]
	name: String,

	#[clap(index = 2)]
	path: String,

	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	// Common options
	#[clap(long = "tags", short = 't')]
	tags: Option<String>,

	#[clap(long)]
	access: Option<config::BuildAccess>,

	#[clap(long)]
	unstable_allow_root: bool,

	#[clap(long)]
	unstable_build_method: Option<config::build::docker::BuildMethod>,

	#[clap(long)]
	unstable_bundle: Option<config::build::docker::BundleKind>,

	#[clap(long)]
	unstable_compression: Option<config::build::Compression>,

	// Docker options
	#[clap(long)]
	docker_image: Option<String>,

	#[clap(long)]
	dockerfile: Option<String>,

	#[clap(long)]
	build_target: Option<String>,

	#[clap(long)]
	build_arg: Option<Vec<String>>,

	// JS options
	#[clap(long)]
	unstable_minify: Option<bool>,

	#[clap(long)]
	unstable_analyze_result: Option<bool>,

	#[clap(long)]
	unstable_esbuild_log_level: Option<String>,

	#[clap(long)]
	manager_enable: Option<bool>,

	#[clap(long)]
	unstable_dump_build: Option<bool>,

	#[clap(long)]
	unstable_no_bundler: Option<bool>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;
		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;
		let bootstrap_data = run_task::<get_bootstrap_data::Task>(
			TaskOutputStyle::None,
			get_bootstrap_data::Input {},
		)
		.await?;
		let cloud_data = bootstrap_data.cloud.as_ref().context("not logged in")?;

		// Find environment
		let environment = match cloud_data.envs.iter().find(|x| x.slug == env) {
			Option::Some(env) => env,
			Option::None => {
				eprintln!("Environment '{env}' not found. Available environments:",);
				for env in &cloud_data.envs {
					eprintln!("- {}", env.slug);
				}
				return Err(errors::GracefulExit.into());
			}
		};

		// Create minimal config
		let build_tags = if let Some(tag_list) = &self.tags {
			Some(kv_str::from_str::<HashMap<String, String>>(&tag_list)?)
		} else {
			None
		};

		// Validate files exist and determine runtime
		let dockerfile_path = self.dockerfile_path();
		let runtime = if self.path.ends_with(".js")
			|| self.path.ends_with(".ts")
			|| self.path.ends_with(".jsx")
			|| self.path.ends_with(".tsx")
		{
			if !Path::new(&self.path).exists() {
				eprintln!("JavaScript/TypeScript file not found: {}", self.path);
				return Err(errors::GracefulExit.into());
			}
			self.create_js_runtime()
		} else if dockerfile_path.exists() {
			self.create_docker_runtime()
		} else {
			let error = format!(
				"Invalid path.\n\nTo publish a JavaScript/TypeScript build, the path must end inany of: .ts, .js, .tsx, .jsx\n\nTo upload a Docker container, a Dockerfile must exist at: {}",
				dockerfile_path.display()
			);
			return Err(errors::UserError::new(error).into());
		};

		// Reserve version name
		let reserve_res =
			apis::cloud_games_versions_api::cloud_games_versions_reserve_version_name(
				&ctx.openapi_config_cloud,
				&ctx.project.game_id.to_string(),
			)
			.await?;
		let version_name = reserve_res.version_display_name;

		// Build and upload
		run_task::<build_publish::Task>(
			TaskOutputStyle::PlainNoResult,
			build_publish::Input {
				environment_id: environment.id,
				build_tags,
				version_name,
				build_name: self.name.clone(),
				runtime,
				access: self.access.clone().unwrap_or(config::BuildAccess::Private),
			},
		)
		.await?;

		Ok(())
	}

	fn dockerfile_path(&self) -> PathBuf {
		if let Some(ref dockerfile) = self.dockerfile {
			PathBuf::from(&self.path).join(dockerfile)
		} else {
			PathBuf::from(&self.path).join("Dockerfile")
		}
	}

	fn create_js_runtime(&self) -> config::build::Runtime {
		config::build::Runtime::JavaScript(config::build::javascript::Build {
			script: self.path.clone(),
			unstable: config::build::javascript::Unstable {
				minify: self.unstable_minify,
				analyze_result: self.unstable_analyze_result,
				esbuild_log_level: self.unstable_esbuild_log_level.clone(),
				compression: self.unstable_compression,
				dump_build: self.unstable_dump_build,
				no_bundler: self.unstable_no_bundler,
			},
		})
	}

	fn create_docker_runtime(&self) -> config::build::Runtime {
		let mut unstable = config::build::docker::Unstable {
			allow_root: Some(self.unstable_allow_root),
			build_method: self.unstable_build_method,
			bundle: self.unstable_bundle,
			compression: self.unstable_compression,
		};

		// Set compression based on bundle type if not explicitly set
		if unstable.compression.is_none() && unstable.bundle.is_some() {
			unstable.compression = Some(config::build::Compression::default_from_bundle_kind(
				unstable.bundle.unwrap(),
			));
		}

		config::build::Runtime::Docker(config::build::docker::Build {
			build_path: Some(self.path.clone()),
			image: self.docker_image.clone(),
			dockerfile: self.dockerfile.clone(),
			build_target: self.build_target.clone(),
			build_args: self.build_arg.as_ref().map(|args| {
				let mut map = HashMap::new();
				for arg in args {
					let parts: Vec<&str> = arg.split('=').collect();
					if parts.len() == 2 {
						map.insert(parts[0].to_string(), parts[1].to_string());
					}
				}
				map
			}),
			unstable: Some(unstable),
		})
	}
}
