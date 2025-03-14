use anyhow::*;
use clap::Parser;
use toolchain::{errors, rivet_api::apis};
use uuid::Uuid;

/// Get details of a specific build
#[derive(Parser)]
pub struct Opts {
	/// The ID of the build to retrieve
	#[clap(index = 1)]
	id: String,

	/// Specify the environment the build is in (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let build_id =
			Uuid::parse_str(&self.id).map_err(|_| errors::UserError::new("invalid id uuid"))?;

		let res = apis::builds_api::builds_get(
			&ctx.openapi_config_cloud,
			&build_id.to_string(),
			Some(&ctx.project.name_id),
			Some(&env),
		)
		.await?;

		println!("{:#?}", res.build);
		Ok(())
	}
}
