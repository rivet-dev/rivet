use anyhow::*;
use bolt_core::{context, tasks};
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
		}

		Ok(())
	}
}
