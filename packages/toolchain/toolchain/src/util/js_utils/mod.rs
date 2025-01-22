use anyhow::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
use tokio::process::Command;

use crate::{paths, util::task};

pub mod schemas;

pub struct CommandOpts {
	pub task_path: &'static str,
	pub input: serde_json::Value,
	pub env: HashMap<String, String>,
}

async fn base_url() -> Result<String> {
	// Attempt to read from user or default
	let base_url = if let Some(url) = std::env::var("_RIVET_JS_UTILS_SRC_DIR").ok() {
		url
	} else {
		rivet_js_utils_embed::src_path(&paths::data_dir()?)
			.await?
			.display()
			.to_string()
	};

	let base_url = base_url.trim_end_matches('/').to_string();
	Ok(base_url)
}

pub struct CommandRaw {
	pub command: PathBuf,
	pub args: Vec<String>,
	pub envs: HashMap<String, String>,
	pub current_dir: PathBuf,
}

pub async fn build_backend_command_raw(opts: CommandOpts) -> Result<CommandRaw> {
	let base_url = base_url().await?;

	// Get Deno executable
	let deno = deno_embed::get_executable(&crate::paths::data_dir()?).await?;

	// Serialize command
	let input_json = serde_json::to_string(&opts.input)?;

	// Run backend
	let mut envs = opts.env;
	envs.insert("DENO_NO_UPDATE_CHECK".into(), "1".into());
	Ok(CommandRaw {
		command: deno.executable_path,
		args: vec![
			"run".into(),
			"--quiet".into(),
			"--no-check".into(),
			"--allow-all".into(),
			"--unstable-sloppy-imports".into(),
			"--vendor".into(),  // Required for unenv files to be readable
			opts.task_path.to_string(),
			"--input".into(),
			input_json,
		],
		envs,
		current_dir: PathBuf::from(base_url),
	})
}

pub async fn build_backend_command(opts: CommandOpts) -> Result<Command> {
	let cmd_raw = build_backend_command_raw(opts).await?;
	let mut cmd = Command::new(cmd_raw.command);
	cmd.kill_on_drop(true);
	cmd.args(cmd_raw.args)
		.envs(cmd_raw.envs)
		.current_dir(cmd_raw.current_dir);

	Ok(cmd)
}

pub async fn run_backend_command_from_task(task: task::TaskCtx, opts: CommandOpts) -> Result<i32> {
	let cmd = build_backend_command(opts).await?;
	let exit_code = task.spawn_cmd(cmd).await?;
	Ok(exit_code.code().unwrap_or(0))
}

pub async fn run_command_and_parse_output<Input: Serialize, Output: DeserializeOwned>(
	js_task_path: &'static str,
	input: &Input,
) -> Result<Output> {
	let input_json =
		serde_json::to_value(input).map_err(|err| anyhow!("Failed to serialize input: {err}"))?;

	let mut cmd = build_backend_command(CommandOpts {
		task_path: js_task_path,
		input: input_json,
		env: HashMap::new(),
	})
	.await
	.map_err(|err| anyhow!("Failed to build command: {err}"))?;

	let output = cmd
		.output()
		.await
		.map_err(|err| anyhow!("Failed to run command: {err}"))?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	if output.status.success() {
		if let Some(last_line) = stdout.lines().rev().find(|line| !line.trim().is_empty()) {
			serde_json::from_str(last_line).map_err(|err| {
				anyhow!("Failed to parse JSON from output: {err}\nOutput: {last_line}")
			})
		} else {
			Err(anyhow!(
				"No non-blank lines in output\nStdout: {stdout}\nStderr: {stderr}"
			))
		}
	} else {
		let mut error_message = format!("Command failed with status: {}", output.status);
		if !stdout.is_empty() {
			error_message.push_str(&format!("\nstdout:\n{stdout}"));
		}
		if !stderr.is_empty() {
			error_message.push_str(&format!("\nstderr:\n{stderr}"));
		}
		Err(anyhow!(error_message))
	}
}
