use anyhow::*;
use bolt_core::{context::ProjectContext, tasks};
use clap::Parser;

#[derive(Parser)]
pub struct LogsOpts {
	#[clap(index = 1)]
	service_name: String,
	#[clap(long, short = 'r')]
	region: Option<String>,
}

impl LogsOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let LogsOpts {
			service_name,
			region,
		} = self;

		tasks::nomad::logs(&ctx, &service_name, region.as_deref().unwrap_or("local")).await?;

		Ok(())
	}
}
