use anyhow::*;
use bolt_core::{
	context::{self, RunContext},
	tasks,
};
use clap::Parser;

#[derive(Parser)]
pub enum SubCommand {
	/// Generates the namespace and secret config.
	///
	/// This can be ran multiple times in case parameters get changed.
	#[clap(alias = "gen")]
	Generate {
		#[clap(index = 1)]
		namespace: String,
	},
	/// Sets the selected namespace in `Bolt.local.toml`.
	#[clap(alias = "set-ns")]
	SetNamespace {
		#[clap(index = 1)]
		namespace: String,
	},
	/// Adds missing regions from supported cloud providers to default_regions.toml.
	#[clap(hide(true))]
	GenerateDefaultRegions,
	ServiceDependencies {
		#[clap(index = 1)]
		svc_name: String,
		#[clap(long, short = 'r')]
		recursive: bool,
		#[clap(long)]
		test: bool,
	},
	Show,
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			Self::Generate { namespace } => {
				let project_root = context::ProjectContextData::seek_project_root().await;
				tasks::config::generate(&project_root, &namespace).await?;
			}
			Self::SetNamespace { namespace } => {
				tasks::config::set_namespace(&namespace).await?;
			}
			Self::GenerateDefaultRegions => tasks::config::generate_default_regions().await?,
			Self::ServiceDependencies {
				svc_name,
				recursive,
				test,
			} => {
				let run_context = if *test {
					RunContext::Test {}
				} else {
					RunContext::Service {}
				};

				// Build project
				let ctx = bolt_core::context::ProjectContextData::new(
					std::env::var("BOLT_NAMESPACE").ok(),
				)
				.await;

				// Read deps
				let deps = if *recursive {
					ctx.recursive_dependencies(&[&svc_name], &run_context).await
				} else {
					let svc = ctx.service_with_name(svc_name).await;
					svc.dependencies(&run_context).await
				};

				// Print deps
				for dep in deps {
					println!("{}", dep.name());
				}
			}
			Self::Show => {
				let ctx = bolt_core::context::ProjectContextData::new(
					std::env::var("BOLT_NAMESPACE").ok(),
				)
				.await;

				println!("{:#?}", ctx.ns());
			}
		}

		Ok(())
	}
}
