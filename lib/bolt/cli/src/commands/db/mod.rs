use anyhow::*;
use bolt_core::{context::ProjectContext, tasks};
use clap::Parser;

mod migrate;

#[derive(Parser)]
pub enum SubCommand {
	Migrate {
		#[clap(subcommand)]
		command: migrate::SubCommand,
	},
	#[clap(alias = "sh")]
	Shell {
		#[clap(index = 1)]
		service: String,
		#[clap(short = 'q', long)]
		query: Option<String>,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Migrate { command } => command.execute(ctx).await,
			Self::Shell { service, query } => {
				tasks::db::shell(
					&ctx,
					&ctx.service_with_name(&service).await,
					query.as_deref(),
				)
				.await?;

				Ok(())
			}
		}
	}
}
