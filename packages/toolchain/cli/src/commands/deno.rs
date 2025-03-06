use anyhow::*;
use clap::Parser;
use serde::Serialize;
use toolchain::{errors, rivet_api::apis, tasks::get_bootstrap_data};

use crate::util::task::{run_task, TaskOutputStyle};

#[derive(Parser, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opts {
	#[clap(long)]
	populate_env: bool,

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
			.env("DENO_NO_UPDATE_CHECK", "1")
			.stdin(std::process::Stdio::inherit())
			.stdout(std::process::Stdio::inherit())
			.stderr(std::process::Stdio::inherit());

		if self.populate_env {
			let bootstrap_data = run_task::<get_bootstrap_data::Task>(
				TaskOutputStyle::None,
				get_bootstrap_data::Input {},
			)
			.await?;
			let ctx = crate::util::login::load_or_login().await?;

			// Find environment
			let env_slug = crate::util::env::get_or_select(&ctx, Option::<String>::None).await?;
			let env = bootstrap_data
				.cloud
				.as_ref()
				.context("not signed in")?
				.envs
				.iter()
				.find(|env| env.slug == env_slug)
				.context("missing env slug")?;

			// Issue service token
			let service_token =
			apis::games_environments_tokens_api::games_environments_tokens_create_service_token(
				&ctx.openapi_config_cloud,
				&ctx.project.game_id.to_string(),
				&env.id.to_string()
			)
			.await?;

			cmd
				// .env("RIVET_ENDPOINT", &ctx.api_endpoint)
				// TODO: Hardcoded
				.env("RIVET_ENDPOINT", "http://74.207.228.118:80")
				.env("RIVET_SERVICE_TOKEN", service_token.token)
				.env("RIVET_PROJECT", &ctx.project.name_id)
				.env("RIVET_ENVIRONMENT", &env.slug);
		}

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
