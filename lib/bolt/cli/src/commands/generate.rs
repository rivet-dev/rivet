use anyhow::*;
use bolt_core::{context::ProjectContext, tasks};
use clap::Parser;

#[derive(Parser, Debug)]
pub enum SubCommand {
	All,
	Project,
	Service {
		#[clap(index = 1)]
		service_name: String,
	},
	#[clap(alias = "as")]
	AllServices,
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::All => {
				tasks::gen::generate_project(&ctx, false).await;
				tasks::gen::generate_all_services(&ctx).await;

				tasks::artifact::generate_project(&ctx).await;
				tasks::artifact::generate_all_services(&ctx).await;
			}
			Self::Project => {
				tasks::gen::generate_project(&ctx, false).await;
				tasks::artifact::generate_project(&ctx).await;
			}
			Self::Service { service_name } => {
				let svc_ctx = ctx.service_with_name(service_name.as_str()).await;
				tasks::gen::generate_service(&svc_ctx).await;
				tasks::artifact::generate_service(&svc_ctx).await;
			}
			Self::AllServices => {
				tasks::gen::generate_all_services(&ctx).await;
				tasks::artifact::generate_all_services(&ctx).await;
			}
		}

		Ok(())
	}
}
