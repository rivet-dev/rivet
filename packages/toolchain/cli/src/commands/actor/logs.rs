use anyhow::*;
use clap::Parser;
use toolchain::errors;
use uuid::Uuid;

/// Stream logs from a specific actor
#[derive(Parser)]
pub struct Opts {
	/// The ID of the actor to stream logs from
	#[clap(index = 1)]
	id: String,

	/// Specify the environment the actor is in (will prompt if not specified)
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	/// Specify which log stream to display (stdout, stderr, or all)
	#[clap(long, short = 's')]
	stream: Option<crate::util::actor::logs::LogStream>,

	/// Disable timestamp display in logs
	#[clap(long)]
	no_timestamps: bool,

	/// Display logs and exit (do not continue following new logs)
	#[clap(long)]
	no_follow: bool,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = crate::util::login::load_or_login().await?;

		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;

		let actor_id =
			Uuid::parse_str(&self.id).map_err(|_| errors::UserError::new("invalid id uuid"))?;

		crate::util::actor::logs::tail(
			&ctx,
			crate::util::actor::logs::TailOpts {
				environment: &env,
				actor_id,
				stream: self
					.stream
					.clone()
					.unwrap_or(crate::util::actor::logs::LogStream::All),
				follow: !self.no_follow,
				timestamps: !self.no_timestamps,
			},
		)
		.await?;

		Ok(())
	}
}
