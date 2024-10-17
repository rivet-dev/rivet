use anyhow::*;
use clap::Parser;

mod provision;

#[derive(Parser)]
pub enum SubCommand {
	Provision(provision::Opts),
}

impl SubCommand {
	pub async fn execute(self) -> Result<()> {
		match self {
			Self::Provision(opts) => opts.execute().await,
		}
	}
}
