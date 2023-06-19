use anyhow::*;
use bolt_core::{context::ProjectContext, dep};
use clap::Parser;

#[derive(Parser)]
pub struct LogsOpts {
	#[clap(index = 1)]
	service_name: String,
	#[clap(long, short = 'f')]
	follow: bool,
	#[clap(long)]
	stderr: bool,
}

impl LogsOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let LogsOpts {
			service_name,
			follow,
			stderr,
		} = self;

		dep::nomad::cli::logs(
			&ctx,
			&service_name,
			&dep::nomad::cli::LogsOpts {
				follow,
				stream: if stderr {
					dep::nomad::cli::LogStream::StdErr
				} else {
					dep::nomad::cli::LogStream::StdOut
				},
			},
		)
		.await?;

		Ok(())
	}
}
