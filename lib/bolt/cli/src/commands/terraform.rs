use anyhow::*;
use bolt_core::{
	context::ProjectContext,
	dep::terraform,
	tasks::{self, gen},
	utils::command_helper::CommandHelper,
};
use clap::Parser;
use duct::cmd;

#[derive(Parser)]
pub enum SubCommand {
	Apply {
		#[clap(index = 1)]
		plan: String,
		#[clap(long, short = 'y')]
		yes: bool,
	},
	Destroy {
		#[clap(index = 1)]
		plan: String,
	},
	Import {
		#[clap(index = 1)]
		plan: String,
		#[clap(index = 2)]
		name: String,
		#[clap(index = 3)]
		id: String,
	},
	#[clap(hide(true))]
	Refresh {
		#[clap(index = 1)]
		plan: String,
	},
	#[clap(hide(true))]
	Graph {
		#[clap(index = 1)]
		plan: String,
		#[clap(short = 'o')]
		output: String,
	},
	#[clap(hide(true))]
	ClearCache {
		#[clap(index = 1)]
		plan: String,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let varfile_path = ctx.gen_tf_env_path();

		match self {
			Self::Apply { plan, yes } => {
				ensure_valid_plan(&ctx, &plan)?;

				gen::generate_project(&ctx).await;

				terraform::cli::apply(&ctx, &plan, yes, &varfile_path).await?;

				terraform::output::clear_cache(&ctx, &plan).await;
			}
			Self::Destroy { plan } => {
				ensure_valid_plan(&ctx, &plan)?;

				gen::generate_project(&ctx).await;

				terraform::cli::destroy(&ctx, &plan, &varfile_path).await?;

				terraform::output::clear_cache(&ctx, &plan).await;
			}
			Self::Import { plan, name, id } => {
				ensure_valid_plan(&ctx, &plan)?;

				gen::generate_project(&ctx).await;

				let mut cmd = terraform::cli::build_command(&ctx, &plan).await;
				cmd.arg("import")
					.arg(format!("-var-file={}", varfile_path.display()))
					.arg(name)
					.arg(id);
				cmd.exec().await?;

				terraform::output::clear_cache(&ctx, &plan).await;
			}
			Self::Refresh { plan } => {
				ensure_valid_plan(&ctx, &plan)?;

				gen::generate_project(&ctx).await;

				let mut cmd = terraform::cli::build_command(&ctx, &plan).await;
				cmd.arg("refresh")
					.arg(format!("-var-file={}", varfile_path.display()));
				cmd.exec().await?;

				terraform::output::clear_cache(&ctx, &plan).await;
			}
			Self::Graph { plan, output } => {
				ensure_valid_plan(&ctx, &plan)?;

				tokio::task::block_in_place(move || {
					cmd!("terraform", "graph")
						.dir(ctx.tf_path().join(plan))
						.pipe(cmd!("dot", "-Tsvg"))
						.stdout_path(output)
						.run()
				})?;
			}
			Self::ClearCache { plan } => {
				terraform::output::clear_cache(&ctx, &plan).await;
			}
		}

		Ok(())
	}
}

/// Validates that the plan ID is included in the setup plan.
///
/// Returns an error if the plan should not be executed.
fn ensure_valid_plan(ctx: &ProjectContext, plan_id: &str) -> Result<()> {
	let all_plans = tasks::infra::all_terraform_plans(ctx)?;

	ensure!(
		all_plans.iter().any(|x| x == plan_id),
		"terraform plan not in setup steps, see `bolt infra plan`. Ensure that all providers in your namespace config are defined."
	);

	Ok(())
}
