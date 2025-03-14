use anyhow::*;
use clap::Parser;
use toolchain::{errors, rivet_api::apis};
use uuid::Uuid;

/// Destroy an existing actor
#[derive(Parser)]
pub struct Opts {
	/// The ID of the actor to destroy
	#[clap(index = 1)]
	id: String,

	/// Specify the environment the actor is in (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Override the kill timeout for the actor (in seconds)
	#[clap(long)]
	override_kill_timeout: Option<i64>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let actor_id =
			Uuid::parse_str(&self.id).map_err(|_| errors::UserError::new("invalid id uuid"))?;

		apis::actors_api::actors_destroy(
			&ctx.openapi_config_cloud,
			&actor_id.to_string(),
			Some(&ctx.project.name_id),
			Some(&env),
			self.override_kill_timeout,
		)
		.await?;

		println!("Destroyed actor: {actor_id}");
		Ok(())
	}
}
