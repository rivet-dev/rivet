use std::{
	collections::BTreeMap,
	io::Write,
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
/// - `max_scale`: integer in `1..=500`
/// - `cpu`: vCPU count in `0.08..=8`; any value in `[0.08, 1)` rounded to two
///   decimals, or exactly `1`, `2`, `4`, or `8`
/// - `memory`: string matching `^(\d+)(Mi|Gi)$` between `512Mi` and `4Gi`
pub fn build_resources(
	cpu: Option<f64>,
	memory: Option<&str>,
	min_scale: Option<u32>,
	max_scale: Option<u32>,
) -> Result<Option<Value>> {
	if cpu.is_none() && memory.is_none() && min_scale.is_none() && max_scale.is_none() {
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
		if !(1..=500).contains(&max_scale) {
			bail!("--max-scale must be between 1 and 500, got {max_scale}");
		}
		resources.insert("maxScale".to_string(), json!(max_scale));
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
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn resources_none_when_all_unset() {
		assert!(build_resources(None, None, None, None).unwrap().is_none());
	}

	#[test]
	fn resources_builds_only_provided_fields() {
		let resources = build_resources(None, None, Some(2), None).unwrap().unwrap();
		assert_eq!(resources, json!({ "minScale": 2 }));
	}

	#[test]
	fn resources_builds_full_object() {
		let resources = build_resources(Some(2.0), Some("1Gi"), Some(1), Some(10))
			.unwrap()
			.unwrap();
		assert_eq!(
			resources,
			json!({ "cpu": 2.0, "memory": "1Gi", "minScale": 1, "maxScale": 10 })
		);
	}

	#[test]
	fn min_scale_bounds() {
		assert!(build_resources(None, None, Some(0), None).is_ok());
		assert!(build_resources(None, None, Some(100), None).is_ok());
		assert!(build_resources(None, None, Some(101), None).is_err());
	}

	#[test]
	fn max_scale_bounds() {
		assert!(build_resources(None, None, None, Some(0)).is_err());
		assert!(build_resources(None, None, None, Some(1)).is_ok());
		assert!(build_resources(None, None, None, Some(500)).is_ok());
		assert!(build_resources(None, None, None, Some(501)).is_err());
	}

	#[test]
	fn cpu_allowed_whole_values() {
		for cpu in [1.0, 2.0, 4.0, 8.0] {
			assert!(build_resources(Some(cpu), None, None, None).is_ok());
		}
	}

	#[test]
	fn cpu_fractional_values() {
		assert!(build_resources(Some(0.08), None, None, None).is_ok());
		assert!(build_resources(Some(0.5), None, None, None).is_ok());
		assert!(build_resources(Some(0.99), None, None, None).is_ok());
		// Below the minimum.
		assert!(build_resources(Some(0.07), None, None, None).is_err());
		// More than two decimals.
		assert!(build_resources(Some(0.123), None, None, None).is_err());
		// Above the range.
		assert!(build_resources(Some(9.0), None, None, None).is_err());
		// In range but not an allowed whole value and not below 1.
		assert!(build_resources(Some(3.0), None, None, None).is_err());
	}

	#[test]
	fn memory_formats_and_bounds() {
		assert!(build_resources(None, Some("512Mi"), None, None).is_ok());
		assert!(build_resources(None, Some("4096Mi"), None, None).is_ok());
		assert!(build_resources(None, Some("1Gi"), None, None).is_ok());
		assert!(build_resources(None, Some("4Gi"), None, None).is_ok());
		// Below minimum.
		assert!(build_resources(None, Some("256Mi"), None, None).is_err());
		// Above maximum.
		assert!(build_resources(None, Some("5Gi"), None, None).is_err());
		// Bad format.
		assert!(build_resources(None, Some("512"), None, None).is_err());
		assert!(build_resources(None, Some("512MB"), None, None).is_err());
		assert!(build_resources(None, Some("Mi"), None, None).is_err());
	}
}
