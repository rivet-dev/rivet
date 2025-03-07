use anyhow::*;
use clap::Parser;
use serde::Serialize;
use std::env;
use std::result::Result::Ok;
use toolchain::{errors, rivet_api::apis, tasks::get_bootstrap_data};

use crate::util::task::{run_task, TaskOutputStyle};

#[derive(Parser, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opts {
	#[clap(
		long,
		short = 'e',
		help = "Execute this command and exit instead of launching an interactive shell"
	)]
	exec: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		eprintln!("[WARNING] shell is experimental");

		// Set up the command based on whether --exec is provided
		let mut cmd = if let Some(exec_command) = &self.exec {
			#[cfg(target_family = "unix")]
			{
				// On Unix-like systems, we use /bin/sh to execute the command
				let mut cmd = tokio::process::Command::new("sh");
				cmd.arg("-c").arg(exec_command);
				cmd
			}

			#[cfg(target_family = "windows")]
			{
				// On Windows, we use cmd.exe
				let mut cmd = tokio::process::Command::new("cmd");
				cmd.arg("/C").arg(exec_command);
				cmd
			}
		} else {
			// No exec command provided, launch an interactive shell
			// Try to determine the current shell from environment variables
			let shell_command = get_shell_command().unwrap_or_else(|| "sh".to_string());

			// Launch the shell with default prompt
			tokio::process::Command::new(&shell_command)
		};

		// Set up standard I/O
		cmd.stdin(std::process::Stdio::inherit())
			.stdout(std::process::Stdio::inherit())
			.stderr(std::process::Stdio::inherit());

		// Always populate environment variables
		let ctx = crate::util::login::load_or_login().await?;
		let env_slug = crate::util::env::get_or_select(&ctx, Option::<String>::None).await?;
		let bootstrap_data = run_task::<get_bootstrap_data::Task>(
			TaskOutputStyle::None,
			get_bootstrap_data::Input {},
		)
		.await?;

		// Find environment
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
				&env.id.to_string(),
			)
			.await?;

		cmd.env("RIVET_ENDPOINT", &ctx.api_endpoint)
			.env("RIVET_SERVICE_TOKEN", service_token.token)
			.env("RIVET_PROJECT", &ctx.project.name_id)
			.env("RIVET_ENVIRONMENT", &env.slug);

		if let Result::Ok(path) = std::env::current_exe() {
			let final_path = std::fs::canonicalize(&path).unwrap_or(path);
			cmd.env("RIVET_CLI_PATH", final_path);
		}

		// Add Deno's env vars
		cmd.env("DENO_NO_UPDATE_CHECK", "1");

		// Determine the proper message based on the command being run
		let process_desc = if let Some(exec_command) = &self.exec {
			format!("command: {}", exec_command)
		} else {
			"shell".to_string()
		};

		let status = cmd
			.status()
			.await
			.context(format!("Failed to wait for {} process", process_desc))?;

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

/// Try to determine the current shell command from environment variables
fn get_shell_command() -> Option<String> {
	// First check SHELL environment variable (Unix systems)
	match env::var("SHELL") {
		Ok(shell) if !shell.is_empty() => {
			// Extract just the shell name from the path
			if let Some(shell_name) = std::path::Path::new(&shell).file_name() {
				if let Some(shell_str) = shell_name.to_str() {
					return Some(shell_str.to_string());
				}
			}
			return Some(shell);
		}
		_ => {}
	}

	// On Windows, check COMSPEC
	match env::var("COMSPEC") {
		Ok(comspec) if !comspec.is_empty() => return Some(comspec),
		_ => {}
	}

	// Check process parent info on Linux systems with /proc
	#[cfg(target_os = "linux")]
	{
		use std::path::Path;
		match std::fs::read_to_string("/proc/self/stat") {
			Ok(ppid) => {
				let parts: Vec<&str> = ppid.split_whitespace().collect();
				if parts.len() > 3 {
					match parts[3].parse::<u32>() {
						Ok(parent_pid) => {
							let cmdline_path = format!("/proc/{}/cmdline", parent_pid);
							match std::fs::read_to_string(cmdline_path) {
								Ok(cmdline) => {
									let cmd = cmdline.split('\0').next().unwrap_or("");
									if !cmd.is_empty() {
										if let Some(cmd_name) = Path::new(cmd).file_name() {
											if let Some(cmd_str) = cmd_name.to_str() {
												if !cmd_str.is_empty() {
													return Some(cmd_str.to_string());
												}
											}
										}
									}
								}
								_ => {}
							}
						}
						_ => {}
					}
				}
			}
			_ => {}
		}
	}

	// Fallback: None (will use "sh" as default)
	None
}
