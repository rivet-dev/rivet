use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Creates a new dev team for an existing team.
	Create { team_id: String },
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Create { team_id } => {
				tasks::api::convert_team(&ctx, team_id).await?;
			}
		}

		utils::ringadingding();

		Ok(())
	}
}
