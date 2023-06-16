use anyhow::*;
use bolt_core::context::ProjectContext;
use bolt_core::dep::salt;
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	Apply {
		#[clap(index = 1)]
		filter: Option<String>,
		#[clap(long)]
		sls: Option<Vec<String>>,
		#[clap(short = 'v', long)]
		verbose: bool,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Apply {
				filter,
				sls,
				verbose,
			} => {
				let apply_opts = salt::cli::ApplyOpts {
					verbose,
					sls,
					..Default::default()
				};
				if let Some(filter) = filter {
					salt::cli::apply(&ctx, &filter, &apply_opts, &Default::default()).await?;
				} else {
					salt::cli::apply_all(&ctx, &apply_opts, &Default::default()).await?;
				}
			}
		}

		Ok(())
	}
}
