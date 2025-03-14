use anyhow::*;
use clap::Parser;
use toolchain::rivet_api::apis;

/// List all available regions for the current project
#[derive(Parser)]
pub struct Opts {
	/// Specify the environment to list regions for (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let res = apis::regions_api::regions_list(
			&ctx.openapi_config_cloud,
			Some(&ctx.project.name_id),
			Some(&env),
		)
		.await
		.context("Failed to list regions")?;

		println!("{:#?}", res.regions);
		Ok(())
	}
}
