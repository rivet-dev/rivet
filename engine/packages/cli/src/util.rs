use std::{
	collections::BTreeMap,
	io::Write,
	path::Path,
	process::{Command as StdCommand, Stdio},
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};

/// URL-encodes a path or query segment.
pub fn encode(value: &str) -> String {
	url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Parses repeated `KEY=VAL` arguments into a map.
pub fn parse_env_vars(vars: &[String]) -> Result<BTreeMap<String, String>> {
	let mut map = BTreeMap::new();
	for var in vars {
		let Some((key, value)) = var.split_once('=') else {
			bail!("--env must be KEY=VAL, got {var}");
		};
		if key.is_empty() {
			bail!("--env key cannot be empty");
		}
		map.insert(key.to_string(), value.to_string());
	}
	Ok(map)
}

/// Default image tag: the current git short SHA, or a unix timestamp outside a
/// git repo.
pub fn default_image_tag() -> String {
	if let Ok(output) = StdCommand::new("git")
		.args(["rev-parse", "--short=7", "HEAD"])
		.stderr(Stdio::null())
		.output()
	{
		if output.status.success() {
			let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
			if !tag.is_empty() {
				return tag;
			}
		}
	}
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
		.to_string()
}

pub fn docker_login(registry: &str, token: &str) -> Result<()> {
	let mut child = StdCommand::new("docker")
		.args(["login", registry, "--username", "rivet", "--password-stdin"])
		.stdin(Stdio::piped())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()
		.context("docker login")?;
	child
		.stdin
		.as_mut()
		.context("docker login stdin unavailable")?
		.write_all(token.as_bytes())?;
	let status = child.wait()?;
	if !status.success() {
		bail!("docker login failed with {status}");
	}
	Ok(())
}

pub fn docker_build(context: &Path, dockerfile: &Path, image_ref: &str) -> Result<()> {
	let context_str = context.to_string_lossy();
	let dockerfile_str = dockerfile.to_string_lossy();
	run_command(
		"docker",
		&[
			"buildx",
			"build",
			"--platform",
			"linux/amd64",
			"--load",
			&context_str,
			"-f",
			&dockerfile_str,
			"-t",
			image_ref,
		],
		None,
	)
}

pub fn run_command(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<()> {
	tracing::info!(command = %format!("{} {}", program, args.join(" ")), "running command");
	let mut command = StdCommand::new(program);
	command
		.args(args)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());
	if let Some(cwd) = cwd {
		command.current_dir(cwd);
	}
	let status = command.status().with_context(|| format!("run {program}"))?;
	if !status.success() {
		bail!("{program} failed with {status}");
	}
	Ok(())
}
