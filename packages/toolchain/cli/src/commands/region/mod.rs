use anyhow::*;
use clap::Subcommand;

mod list;

/// Commands for managing regions
#[derive(Subcommand)]
pub enum SubCommand {
	/// List all available regions for the current project
	List(list::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::List(opts) => opts.execute().await,
		}
	}
}
