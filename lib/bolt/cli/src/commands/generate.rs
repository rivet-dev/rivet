use anyhow::*;
use bolt_core::{context::ProjectContext, tasks};
use clap::Parser;

#[derive(Parser, Debug)]
pub enum SubCommand {
	Project,
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Project => {
				tasks::gen::generate_project(&ctx, false).await;
			}
		}

		Ok(())
	}
}
