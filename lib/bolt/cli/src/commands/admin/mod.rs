use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

mod team_dev;

#[derive(Parser)]
pub enum SubCommand {
	TeamDev {
		#[clap(subcommand)]
		command: team_dev::SubCommand,
	},
	Login {
		#[clap(default_value = "root")]
		name: String,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::TeamDev { command } => command.execute(ctx).await,
			Self::Login { name } => {
				tasks::api::access_token_login(&ctx, name).await?;

				utils::ringadingding();

				Ok(())
			}
		}
	}
}
