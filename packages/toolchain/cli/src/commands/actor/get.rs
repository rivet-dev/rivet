use anyhow::*;
use clap::Parser;
use toolchain::rivet_api::apis;
use uuid::Uuid;

/// Get details of a specific actor
#[derive(Parser)]
pub struct Opts {
	/// The ID of the actor to retrieve
	#[clap(index = 1)]
	id: String,

	/// Specify the environment the actor is in (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let actor_id = Uuid::parse_str(&self.id).context("invalid id uuid")?;

		let res = apis::actors_api::actors_get(
			&ctx.openapi_config_cloud,
			&actor_id.to_string(),
			Some(&ctx.project.name_id),
			Some(&env),
			None,
		)
		.await?;

		println!("{:#?}", res.actor);
		Ok(())
	}
}
