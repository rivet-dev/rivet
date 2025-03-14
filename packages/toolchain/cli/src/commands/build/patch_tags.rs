use anyhow::*;
use clap::Parser;
use std::collections::HashMap;
use toolchain::rivet_api::{apis, models};

/// Update tags for a specific build
#[derive(Parser)]
pub struct Opts {
	/// The ID of the build to update tags for
	#[clap(index = 1)]
	build: String,

	/// Specify the environment the build is in (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Tags to set on the build (key=value format)
	#[clap(short = 't', long = "tag")]
	tags: Option<String>,

	/// Comma-separated list of tag keys to make exclusive (will remove these tags from other builds)
	#[clap(short = 'e', long = "exclusive-tags")]
	exclusive_tags: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		// Parse tags
		let tags = self
			.tags
			.as_ref()
			.map(|tags_str| kv_str::from_str::<HashMap<String, String>>(tags_str))
			.transpose()?;
		let exclusive_tags = self.exclusive_tags.as_ref().map(|x| {
			x.split(",")
				.map(|x| x.trim().to_string())
				.collect::<Vec<String>>()
		});

		apis::builds_api::builds_patch_tags(
			&ctx.openapi_config_cloud,
			&self.build,
			models::BuildsPatchBuildTagsRequest {
				tags: tags.map(|x| serde_json::json!(x)),
				exclusive_tags,
			},
			Some(&ctx.project.name_id),
			Some(&env),
		)
		.await?;

		println!("Patched tags");
		Ok(())
	}
}
