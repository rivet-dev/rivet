
use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::Parser;

/// Used to extract data from the Bolt configs. This gets called primarily in
/// shell scripts.
///
/// Bolt is intended to be the single source of truth, so this lets other tools
/// extract relevant information.
#[derive(Parser, Debug)]
pub enum SubCommand {
	Namespace,
	ProjectRoot,
	ServiceName {
		#[clap(index = 1, action = clap::ArgAction::Append)]
		service_names: Vec<String>,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Namespace => {
				println!("{}", ctx.ns_id());
			}
			Self::ProjectRoot => {
				print!("{}", ctx.path().display());
			}
			Self::ServiceName { service_names } => {
				for svc_ctx in ctx.services_with_patterns(&service_names).await {
					println!("{}", svc_ctx.name());
				}
			}
		}
		Ok(())
	}
}
