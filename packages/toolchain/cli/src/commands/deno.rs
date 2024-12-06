use anyhow::*;
use clap::Parser;
use serde::Serialize;
use toolchain::errors;

#[derive(Parser, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opts {
	#[clap(trailing_var_arg = true)]
	#[clap(allow_hyphen_values = true)]
	#[clap(value_parser)]
	args: Vec<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let deno = deno_embed::get_executable(&toolchain::paths::data_dir()?).await?;

		let mut cmd = tokio::process::Command::new(&deno.executable_path);
		cmd.args(&self.args)
			.stdin(std::process::Stdio::inherit())
			.stdout(std::process::Stdio::inherit())
			.stderr(std::process::Stdio::inherit());
		if let Result::Ok(path) = std::env::current_exe() {
			let final_path = std::fs::canonicalize(&path).unwrap_or(path);
			cmd.env("RIVET_CLI_PATH", final_path);
		}
		let status = cmd
			.status()
			.await
			.context("Failed to wait for deno process")?;

		if !status.success() {
			let code = std::process::ExitCode::from(
				status
					.code()
					.and_then(|x| u8::try_from(x).ok())
					.unwrap_or(1),
			);
			return Err(errors::PassthroughExitCode::new(code).into());
		}

		Ok(())
	}
}
