use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::Parser;
use tokio::process::Command;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		Command::new("kubectl")
			.args([
				"exec",
				"-it",
				"-n",
				"rivet-service",
				&format!("deployment/rivet-shell"),
				"--",
				"/bin/bash",
			])
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.status()
			.await?;

		Ok(())
	}
}
