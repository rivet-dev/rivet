use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct UpOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,

	#[clap(long)]
	load_tests: bool,
}

impl UpOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let UpOpts {
			service_names,
			load_tests,
		} = self;

		// Bring up the service
		if !service_names.is_empty() {
			tasks::up::up_services(&ctx, &service_names, load_tests).await?;
		} else {
			tasks::up::up_all(&ctx, load_tests).await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
