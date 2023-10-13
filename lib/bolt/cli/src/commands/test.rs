use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct TestOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	#[clap(long)]
	filter: Vec<String>,
	/// Amount of time until forced test timeout (in seconds).
	#[clap(long, short = 't')]
	timeout: Option<u64>,
	/// Amount of parallel tests to run simultaneously.
	#[clap(long, short = 'c')]
	parallel_tests: Option<usize>,
	/// Don't purge Nomad jobs after test completion.
	#[clap(long)]
	no_purge: bool,
}

impl TestOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let TestOpts {
			service_names,
			filter,
			timeout,
			parallel_tests,
			no_purge,
		} = self;

		if let Some(parallel_tests) = parallel_tests {
			ensure!(parallel_tests <= 100, "too many parallel tests (> 100)");
		}

		// Test services
		if !service_names.is_empty() {
			tasks::test::test_services(
				&ctx,
				tasks::test::TestCtx {
					svc_names: &service_names,
					filters: filter,
					timeout,
					parallel_tests,
					no_purge,
				},
			)
			.await?;
		} else {
			ensure!(
				filter.is_empty(),
				"cannot provide filters when testing all services"
			);
			tasks::test::test_all(&ctx).await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
