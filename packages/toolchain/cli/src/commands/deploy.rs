use anyhow::*;
use clap::Parser;
use std::collections::HashMap;

/// Deploy a build to a specific environment
#[derive(Parser)]
pub struct Opts {
	/// Specify the environment to deploy to (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Filter which builds to deploy by tags (key=value comma-separated format)
	#[clap(long)]
	filter_tags: Option<String>,

	/// Extra tags to add to the build (key=value comma-separated format)
	#[clap(long)]
	extra_tags: Option<String>,

	#[clap(long, help = "Override the automatically generated version name")]
	version: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let filter_tags = self
			.filter_tags
			.as_ref()
			.map(|b| kv_str::from_str::<HashMap<String, String>>(b))
			.transpose()
			.context("Failed to parse filter tags")?;

		let build_tags = self
			.extra_tags
			.as_ref()
			.map(|b| kv_str::from_str::<HashMap<String, String>>(b))
			.transpose()
			.context("Failed to parse extra tags")?;

		crate::util::deploy::deploy(crate::util::deploy::DeployOpts {
			environment: &env,
			filter_tags: filter_tags,
			build_tags: build_tags,
			version: self.version.clone(),
		})
		.await?;

		Ok(())
	}
}
