use std::{path::PathBuf, process::Stdio};

use anyhow::{Context, Result, bail};
use clap::Parser;
use rivetkit_engine_process::{engine_env, resolve_engine_binary_path};
use tokio::process::Command;

use crate::engine_runner::engine_config;

#[derive(Parser)]
pub struct Opts {
	/// Path to a rivet-engine binary. Defaults to RIVET_ENGINE_BINARY_PATH, a
	/// binary next to this CLI, a local build, or an auto-downloaded release.
	#[arg(long)]
	engine_binary: Option<PathBuf>,
	/// Arguments forwarded verbatim to the rivet-engine binary.
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	args: Vec<String>,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let config = engine_config(self.engine_binary);
		let binary = resolve_engine_binary_path(&config).await?;
		let env = engine_env(&config)?;

		let mut command = Command::new(&binary);
		command.args(&self.args);
		for (key, value) in &env {
			command.env(key, value);
		}
		command
			.stdin(Stdio::inherit())
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit());

		let status = command
			.status()
			.await
			.with_context(|| format!("run {}", binary.display()))?;
		if !status.success() {
			bail!("rivet-engine exited with {status}");
		}
		Ok(())
	}
}
