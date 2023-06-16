use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	tasks::{self, infra::ExecutePlanOpts},
};
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints out the plan used to provision the Rivet cluster.
	Plan,
	/// Provisions all infrastructure required for the Rivet cluster.
	Up {
		#[clap(long, short = 'y')]
		yes: bool,
	},
	/// Destroys all provisioned infrastructure.
	Destroy {
		#[clap(long, short = 'y')]
		yes: bool,
	},
	/// Manages infrastructure migrations.
	#[clap(hide(true))]
	Migrate {
		#[clap(subcommand)]
		command: MigrateSubCommand,
	},
}

#[derive(Parser)]
pub enum MigrateSubCommand {
	BreakInfraTerraformMonolith,
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Plan => {
				let plan = tasks::infra::build_plan(&ctx)?;
				println!("{plan:#?}");
			}
			Self::Up { yes: auto_approve } => {
				let plan = tasks::infra::build_plan(&ctx)?;
				tasks::infra::execute_plan(&ctx, &plan, ExecutePlanOpts { auto_approve }).await?;
			}
			Self::Destroy { yes: auto_approve } => {
				let plan = tasks::infra::build_plan(&ctx)?;
				tasks::infra::destroy_plan(&ctx, &plan, ExecutePlanOpts { auto_approve }).await?;
			}
			Self::Migrate { command } => match command {
				MigrateSubCommand::BreakInfraTerraformMonolith => {
					tasks::infra::migrate::break_infra_terraform_monolith::run(&ctx).await?;
				}
			},
		}

		Ok(())
	}
}
