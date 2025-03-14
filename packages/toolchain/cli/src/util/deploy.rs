use anyhow::*;
use std::collections::HashMap;
use toolchain::{
	errors,
	tasks::{deploy, get_bootstrap_data},
};
use uuid::Uuid;

use crate::util::task::{run_task, TaskOutputStyle};

pub struct DeployOpts<'a> {
	pub environment: &'a str,
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

	let build = run_task::<deploy::Task>(
		TaskOutputStyle::PlainNoResult,
		deploy::Input {
			config,
			environment_id: environment.id,
			build_tags: opts.build_tags,
			version_name: opts.version,
		},
	)
	.await?;

	Ok(build.build_ids)
}
