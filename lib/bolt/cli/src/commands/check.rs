use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct CheckOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	#[clap(long, short = 'g')]
	skip_generate: bool,
	#[clap(long, short = 't')]
	skip_tests: bool,
	/// Skip 1Password config sync check.
	#[clap(long, short = 's')]
	skip_config_sync_check: bool,
	#[clap(long)]
	validate_format: bool,
}

impl CheckOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let CheckOpts {
			service_names,
			skip_generate,
			skip_tests,
			skip_config_sync_check,
			validate_format,
		} = self;

		if !service_names.is_empty() {
			tasks::check::check_service(
				&ctx,
				&service_names,
				false,
				skip_generate,
				skip_tests,
				skip_config_sync_check,
				validate_format,
			)
			.await;
		} else {
			tasks::check::check_all(
				&ctx,
				false,
				skip_generate,
				skip_tests,
				skip_config_sync_check,
				validate_format,
			)
			.await;
		}

		utils::ringadingding();

		Ok(())
	}
}
