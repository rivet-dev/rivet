use anyhow::*;
use bolt_core::{
	context,
	tasks::{self, infra::ExecutePlanOpts},
};
use clap::Parser;

#[derive(Parser)]
pub struct InitOpts {
	#[clap(index = 1)]
	namespace: String,
	#[clap(long, short = 'y')]
	yes: bool,
}

impl InitOpts {
	pub async fn execute(&self) -> Result<()> {
		// Generate config
		let project_root = bolt_core::context::ProjectContextData::seek_project_root().await;
		bolt_core::tasks::config::generate(&project_root, &self.namespace).await?;

		// Set namespace
		tasks::config::set_namespace(&self.namespace).await?;

		// Build project context
		let ctx = context::ProjectContextData::new(Some(self.namespace.clone())).await;

		// Apply infra
		let plan = tasks::infra::build_plan(&ctx, None, false)?;
		tasks::infra::execute_plan(
			&ctx,
			&plan,
			ExecutePlanOpts {
				auto_approve: self.yes,
			},
		)
		.await?;

		Ok(())
	}
}
