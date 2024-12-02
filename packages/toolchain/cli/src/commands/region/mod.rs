use anyhow::*;
use clap::Subcommand;

mod list;

#[derive(Subcommand)]
pub enum SubCommand {
	List(list::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::List(opts) => opts.execute().await,
		}
	}
}
