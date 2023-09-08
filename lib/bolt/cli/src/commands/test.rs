use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct TestOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	// #[clap(short = 'n', long = "name")]
	// test_name: Option<String>,
}

impl TestOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let TestOpts { service_names } = self;

		// Test services
		if !service_names.is_empty() {
			tasks::test::test_services(&ctx, &service_names).await?;
		} else {
			tasks::test::test_all(&ctx).await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
