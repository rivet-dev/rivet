use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct UpOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	#[clap(long)]
	load_tests: bool,
	#[clap(long)]
	build_only: bool,
	/// Builds and uploads containers (if distributed), but does not deploy.
	#[clap(long)]
	skip_deploy: bool,
	/// Skip 1Password config sync check.
	#[clap(long, short = 's')]
	skip_config_sync_check: bool,
}

impl UpOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let UpOpts {
			service_names,
			load_tests,
			build_only,
			skip_deploy,
			skip_config_sync_check,
		} = self;

		// Bring up the service
		if !service_names.is_empty() {
			tasks::up::up_services(
				&ctx,
				&service_names,
				load_tests,
				build_only,
				skip_deploy,
				skip_config_sync_check,
			)
			.await?;
		} else {
			tasks::up::up_all(
				&ctx,
				load_tests,
				build_only,
				skip_deploy,
				skip_config_sync_check,
			)
			.await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
