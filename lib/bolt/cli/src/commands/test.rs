use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct TestOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	#[clap(long, short = 't')]
	test_only: bool,
	#[clap(long, short = 'd')]
	skip_dependencies: bool,
	#[clap(short = 'n', long = "name")]
	test_name: Option<String>,
	#[clap(long, short = 'f')]
	force_build: bool,
	#[clap(long, short = 'g')]
	skip_generate: bool,
}

impl TestOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let TestOpts {
			service_names,
			test_only,
			skip_dependencies,
			test_name,
			force_build,
			skip_generate,
		} = self;

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
