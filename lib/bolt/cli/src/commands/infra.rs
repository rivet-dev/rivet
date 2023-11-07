use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	tasks::{self, infra::ExecutePlanOpts},
};
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Prints out the plan used to provision the Rivet cluster.
	Plan {
		#[clap(long)]
		start_at: Option<String>,
	},
	/// Provisions all infrastructure required for the Rivet cluster.
	Up {
		#[clap(long, short = 'y')]
		yes: bool,
		#[clap(long)]
		start_at: Option<String>,
	},
	/// Destroys all provisioned infrastructure.
	Destroy {
		#[clap(long, short = 'y')]
		yes: bool,
		#[clap(long)]
		start_at: Option<String>,
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
	// Placeholder
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Plan { start_at } => {
				let plan = tasks::infra::build_plan(&ctx, start_at, false)?;
				for step in plan {
					println!("{}: {:?}", step.name_id, step.kind);
				}
			}
			Self::Up {
				yes: auto_approve,
				start_at,
			} => {
				let plan = tasks::infra::build_plan(&ctx, start_at, false)?;
				tasks::infra::execute_plan(&ctx, &plan, ExecutePlanOpts { auto_approve }).await?;
			}
			Self::Destroy {
				yes: auto_approve,
				start_at,
			} => {
				let plan = tasks::infra::build_plan(&ctx, start_at, true)?;
				tasks::infra::destroy_plan(&ctx, &plan, ExecutePlanOpts { auto_approve }).await?;
			}
			Self::Migrate { command } => match command {
				// Placeholder
			},
		}

		Ok(())
	}
}
