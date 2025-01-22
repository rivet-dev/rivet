use anyhow::*;
use clap::Parser;
use toolchain::{errors, rivet_api::apis};
use uuid::Uuid;

#[derive(Parser)]
pub struct Opts {
	#[clap(index = 1)]
	id: String,

	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let build_id =
			Uuid::parse_str(&self.id).map_err(|_| errors::UserError::new("invalid id uuid"))?;

		let res = apis::actor_builds_api::actor_builds_get(
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
