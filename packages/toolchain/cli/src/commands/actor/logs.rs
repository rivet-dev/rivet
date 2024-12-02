use anyhow::*;
use clap::Parser;
use toolchain::errors;
use uuid::Uuid;

#[derive(Parser)]
pub struct Opts {
	#[clap(index = 1)]
	id: String,

	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,

	#[clap(long, short = 's')]
	stream: Option<crate::util::actor::logs::LogStream>,

	#[clap(long)]
	no_timestamps: bool,

	#[clap(long)]
	no_follow: bool,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = toolchain::toolchain_ctx::load().await?;

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
