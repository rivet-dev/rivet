use std::{fs, path::PathBuf};

use anyhow::{Result, bail};
use clap::Parser;

use crate::templates::{RIVET_DEPLOY_WORKFLOW_PATH, rivet_deploy_workflow};

#[derive(Parser)]
pub struct Opts {
	/// Overwrite the workflow file if it already exists.
	#[arg(long)]
	force: bool,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let path = PathBuf::from(RIVET_DEPLOY_WORKFLOW_PATH);
		if path.exists() && !self.force {
			bail!(
				"{} already exists; pass --force to overwrite",
				path.display()
			);
		}
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)?;
		}
		fs::write(&path, rivet_deploy_workflow())?;
		tracing::info!(path = %path.display(), "wrote GitHub Actions deploy workflow");
		tracing::info!("add your Rivet Cloud token as a repository secret to enable CI:");
		tracing::info!("  gh secret set RIVET_CLOUD_TOKEN");
		Ok(())
	}
}
