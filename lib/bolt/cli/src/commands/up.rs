use anyhow::*;
use bolt_core::{context::ProjectContext, tasks, utils};
use clap::Parser;

#[derive(Parser)]
pub struct UpOpts {
	#[clap(index = 1, action = clap::ArgAction::Append)]
	service_names: Vec<String>,
	#[clap(long)]
	skip_build: bool,
	#[clap(long, short = 'd')]
	skip_dependencies: bool,
	#[clap(long, short = 'f')]
	force_build: bool,
	#[clap(long, short = 'g')]
	skip_generate: bool,
	#[clap(long, short = 'y')]
	yes: bool,
}

impl UpOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let UpOpts {
			service_names,
			skip_build,
			skip_dependencies,
			force_build,
			skip_generate,
			yes: auto_approve,
		} = self;

		// Bring up the service
		if !service_names.is_empty() {
			tasks::up::up_services(
				&ctx,
				&service_names,
				tasks::up::UpOpts {
					skip_build,
					skip_dependencies,
					force_build,
					skip_generate,
					auto_approve,
				},
			)
			.await?;
		} else {
			tasks::up::up_all(
				&ctx,
				tasks::up::UpOpts {
					skip_build,
					skip_dependencies,
					force_build,
					skip_generate,
					auto_approve,
				},
			)
			.await?;
		}

		utils::ringadingding();

		Ok(())
	}
}
