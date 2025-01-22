use anyhow::*;
use clap::Parser;
use std::collections::HashMap;

#[derive(Parser)]
pub struct Opts {
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	#[clap(long, short = 't')]
	tags: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let build_tags = self
			.tags
			.as_ref()
			.map(|b| kv_str::from_str::<HashMap<String, String>>(b))
			.transpose()
			.context("Failed to parse build tags")?;

		crate::util::deploy::deploy(crate::util::deploy::DeployOpts {
			environment: &env,
			build_tags,
		})
		.await?;

		Ok(())
	}
}
