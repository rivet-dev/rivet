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

		if !service_names.is_empty() {
			tasks::test::test_service(
				&ctx,
				&service_names,
				test_only,
				test_name.as_deref(),
				skip_dependencies,
				force_build,
				skip_generate,
			)
			.await?;
		} else {
			tasks::test::test_all(
				&ctx,
				test_only,
				test_name.as_deref(),
				force_build,
				skip_generate,
			)
			.await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
