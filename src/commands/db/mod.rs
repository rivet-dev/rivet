use anyhow::*;
use clap::Parser;

mod migrate;

#[derive(Parser)]
pub enum SubCommand {
	Migrate {
		#[clap(subcommand)]
		command: migrate::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self) -> Result<()> {
		match self {
			Self::Migrate { command } => command.execute().await,
		}
	}
}
