use anyhow::*;
use clap::Subcommand;

mod select;

#[derive(Subcommand)]
pub enum SubCommand {
	#[clap(alias = "s")]
	Select(select::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Select(opts) => opts.execute().await,
		}
	}
}
