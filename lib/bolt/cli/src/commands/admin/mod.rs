use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::Parser;

mod team_dev;

#[derive(Parser)]
pub enum SubCommand {
	TeamDev {
		#[clap(subcommand)]
		command: team_dev::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::TeamDev { command } => command.execute(ctx).await,
		}
	}
}
