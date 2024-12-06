use anyhow::*;
use clap::Subcommand;

mod endpoint;

#[derive(Subcommand)]
pub enum SubCommand {
	#[clap(alias = "e")]
	Endpoint(endpoint::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Endpoint(opts) => opts.execute().await,
		}
	}
}
