use anyhow::*;
use clap::Parser;
use std::collections::HashMap;
use toolchain::rivet_api::apis;

/// List all builds in the current project
#[derive(Parser)]
pub struct Opts {
	/// Specify the environment to list builds from (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Filter builds by tags (key=value format)
	#[clap(long)]
	tags: Option<String>,
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
		let tags_json = tags.map(|t| serde_json::to_string(&t)).transpose()?;

		let res = apis::builds_api::builds_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id),
			Some(&env),
			tags_json.as_deref(),
		)
		.await?;

		println!("{:#?}", res.builds);
		Ok(())
	}
}
