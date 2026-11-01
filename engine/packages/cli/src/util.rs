use std::{
	collections::BTreeMap,
	io::{IsTerminal, Write},
	path::Path,
	process::{Command as StdCommand, Stdio},
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// URL-encodes a path or query segment.
pub fn encode(value: &str) -> String {
	url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Reports whether colored output should be used. Color is on by default and
/// disabled when `NO_COLOR` is set to a non-empty value or stdout is not a TTY.
pub fn color_enabled() -> bool {
	let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
	!no_color && std::io::stdout().is_terminal()
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

/// Validates the optional compute resource flags and builds the `resources`
/// object for the managed-pool upsert body. Returns `Ok(None)` when every flag
/// is unset so the server applies its own defaults.
///
/// The accepted ranges mirror the server-side `computePoolConfigResources`
/// schema:
/// - `min_scale`: integer in `0..=100`
/// - `max_scale`: integer in `1..=5000`
/// - `instance_request_concurrency`: integer in `1..=500`
/// - `cpu`: vCPU count in `0.08..=8`; any value in `[0.08, 1)` rounded to two
///   decimals, or exactly `1`, `2`, `4`, or `8`
/// - `memory`: string matching `^(\d+)(Mi|Gi)$` between `512Mi` and `4Gi`
pub fn build_resources(
	cpu: Option<f64>,
	memory: Option<&str>,
	min_scale: Option<u32>,
	max_scale: Option<u32>,
	instance_request_concurrency: Option<u32>,
) -> Result<Option<Value>> {
	if cpu.is_none()
		&& memory.is_none()
		&& min_scale.is_none()
		&& max_scale.is_none()
		&& instance_request_concurrency.is_none()
	{
		return Ok(None);
	}

	let mut resources = serde_json::Map::new();

	if let Some(min_scale) = min_scale {
		if min_scale > 100 {
			bail!("--min-scale must be between 0 and 100, got {min_scale}");
		}
		resources.insert("minScale".to_string(), json!(min_scale));
	}

	if let Some(max_scale) = max_scale {
		if !(1..=5000).contains(&max_scale) {
			bail!("--max-scale must be between 1 and 5000, got {max_scale}");
		}
		resources.insert("maxScale".to_string(), json!(max_scale));
	}

	if let Some(instance_request_concurrency) = instance_request_concurrency {
		if !(1..=500).contains(&instance_request_concurrency) {
			bail!(
				"--instance-request-concurrency must be between 1 and 500, got {instance_request_concurrency}"
			);
		}
		resources.insert(
			"instanceRequestConcurrency".to_string(),
			json!(instance_request_concurrency),
		);
	}

	if let Some(cpu) = cpu {
		if !cpu.is_finite() {
			bail!("--cpu must be a finite number, got {cpu}");
		}
		if !(0.08..=8.0).contains(&cpu) {
			bail!("--cpu must be between 0.08 and 8, got {cpu}");
		}
		// Valid values are exactly 1, 2, 4, or 8 vCPUs, or any value in
		// [0.08, 1) expressible with two decimal places.
		let is_whole = cpu == 1.0 || cpu == 2.0 || cpu == 4.0 || cpu == 8.0;
		let scaled = cpu * 100.0;
		let is_two_decimals = cpu < 1.0 && (scaled - scaled.round()).abs() < 1e-9;
		if !is_whole && !is_two_decimals {
			bail!(
				"--cpu must be 1, 2, 4, or 8, or a value in [0.08, 1) with at most two decimals, got {cpu}"
			);
		}
		resources.insert("cpu".to_string(), json!(cpu));
	}

	if let Some(memory) = memory {
		let mib = parse_memory_mib(memory)?;
		if !(512..=4096).contains(&mib) {
			bail!("--memory must be between 512Mi and 4Gi, got {memory}");
		}
		resources.insert("memory".to_string(), json!(memory));
	}

	Ok(Some(Value::Object(resources)))
}

/// Validates the optional runner-config flags and builds the `runnerConfig`
/// object for the managed-pool upsert body. Returns `Ok(None)` when every flag
/// is unset so the server keeps the pool's current runner config.
///
/// The accepted ranges mirror the server-side
/// `computePoolConfigRunnerConfigOverride` schema:
/// - `max_concurrent_actors`: integer in `1..=50000`
/// - `drain_grace_period`: seconds, integer in `5..=3200`
pub fn build_runner_config(
	max_concurrent_actors: Option<u32>,
	drain_grace_period: Option<u32>,
	drain_on_version_upgrade: Option<bool>,
) -> Result<Option<Value>> {
	if max_concurrent_actors.is_none()
		&& drain_grace_period.is_none()
		&& drain_on_version_upgrade.is_none()
	{
		return Ok(None);
	}

	let mut runner_config = serde_json::Map::new();

	if let Some(max_concurrent_actors) = max_concurrent_actors {
		if !(1..=50000).contains(&max_concurrent_actors) {
			bail!(
				"--max-concurrent-actors must be between 1 and 50000, got {max_concurrent_actors}"
			);
		}
		runner_config.insert(
			"maxConcurrentActors".to_string(),
			json!(max_concurrent_actors),
		);
	}

	if let Some(drain_grace_period) = drain_grace_period {
		if !(5..=3200).contains(&drain_grace_period) {
			bail!("--drain-grace-period must be between 5 and 3200, got {drain_grace_period}");
		}
		runner_config.insert("drainGracePeriod".to_string(), json!(drain_grace_period));
	}

	if let Some(drain_on_version_upgrade) = drain_on_version_upgrade {
		runner_config.insert(
			"drainOnVersionUpgrade".to_string(),
			json!(drain_on_version_upgrade),
		);
	}

	Ok(Some(Value::Object(runner_config)))
}

/// Parses a memory string of the form `^(\d+)(Mi|Gi)$` into mebibytes.
fn parse_memory_mib(memory: &str) -> Result<u64> {
	let (digits, multiplier) = if let Some(digits) = memory.strip_suffix("Mi") {
		(digits, 1)
	} else if let Some(digits) = memory.strip_suffix("Gi") {
		(digits, 1024)
	} else {
		bail!("--memory must match <number>Mi or <number>Gi, got {memory}");
	};
	if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
		bail!("--memory must match <number>Mi or <number>Gi, got {memory}");
	}
	let value: u64 = digits
		.parse()
		.with_context(|| format!("invalid memory value: {memory}"))?;
	Ok(value * multiplier)
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

// These tests live inline because `rivet-cli` is a binary-only crate, so its
// items cannot be reached from an integration test under `tests/`.
