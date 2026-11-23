//! Deploys to a BYOC cluster: the image is uploaded as a build through the
//! Cloud API, and the pool is applied by an operator command the Cloud API
//! queues, which this module waits on until the operator reports it.

use std::{
	collections::BTreeMap,
	fs::File,
	io::Read,
	path::{Path, PathBuf},
	process::Command,
	time::Duration,
};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::time::{Instant, sleep};

use crate::{
	cloud::{
		CloudClient,
		v2::{
			self, ByocBuild, ByocBuildStatus, ByocCommand, ByocCommandStatus, ByocPoolApply,
			ByocResources, NewByocBuild,
		},
	},
	cluster_env::validate_env_name,
	util::{default_image_tag, docker_build, run_command},
};

/// The largest tar the Cloud API accepts for a BYOC build, uploaded in one PUT.
const MAX_BUILD_BYTES: u64 = 5000 * 1024 * 1024;
const COMMAND_POLL_INTERVAL: Duration = Duration::from_secs(5);
/// How often a command that has not changed is reported as still waiting.
const COMMAND_PROGRESS_INTERVAL: Duration = Duration::from_secs(30);
/// The Cloud API decides when a command has timed out (it reports
/// `timed_out`); this cap only keeps the CLI from waiting forever should the
/// server never do so, and is well past every server-side limit.
const COMMAND_SAFETY_CAP: Duration = Duration::from_secs(2 * 60 * 60);
/// The longest pool name the Cloud API accepts, a DNS label.
const MAX_POOL_NAME_CHARS: usize = 52;
const MAX_REPLICAS: u32 = 1000;

pub struct ByocDeploy {
	pub cluster: String,
	pub pool: String,
	pub build: Option<String>,
	pub dockerfile: PathBuf,
	pub build_context: PathBuf,
	pub image: Option<String>,
	pub tag: Option<String>,
	pub pool_apply: PoolSettings,
}

/// The flags that make up a BYOC pool, checked before any build work.
pub struct PoolSettings {
	/// `None` when neither `--cpu` nor `--memory` is given.
	pub resources: Option<ByocResources>,
	pub min_replicas: Option<u32>,
	pub max_replicas: Option<u32>,
	pub env: Option<BTreeMap<String, String>>,
	pub secret_env: Option<BTreeMap<String, String>>,
	pub pool_config: Option<Map<String, Value>>,
}

pub struct ByocPrepare {
	pub cluster: String,
	pub dockerfile: PathBuf,
	pub build_context: PathBuf,
	pub image: Option<String>,
	pub tag: Option<String>,
}

/// The size and digests of a build's tar, read in one pass.
#[derive(Debug, PartialEq, Eq)]
pub struct TarDigest {
	pub content_length: u64,
	/// Base64, as `Content-MD5` carries it.
	pub content_md5: String,
	/// Lowercase hex.
	pub sha256: String,
}

/// Checks the pool name as the Cloud API does, so a bad one fails before a
/// long build rather than after it.
pub fn validate_pool_name(pool: &str) -> Result<()> {
	let bytes = pool.as_bytes();
	let valid = !bytes.is_empty()
		&& bytes.len() <= MAX_POOL_NAME_CHARS
		&& bytes
			.iter()
			.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
		&& bytes.first() != Some(&b'-')
		&& bytes.last() != Some(&b'-');
	if !valid {
		bail!(
			"pool name {pool:?} is invalid; use lowercase letters, digits and `-`, at most {MAX_POOL_NAME_CHARS} characters, not starting or ending with `-`"
		);
	}
	Ok(())
}

/// Whether `value` reads as a Kubernetes quantity the Cloud API accepts:
/// digits and dots, then an optional unit suffix.
fn quantity_like(value: &str) -> bool {
	let number = value.trim_end_matches(|c: char| c.is_ascii_alphabetic());
	let suffix = &value[number.len()..];
	!number.is_empty()
		&& number.bytes().all(|b| b.is_ascii_digit() || b == b'.')
		&& matches!(
			suffix,
			"" | "m" | "k" | "M" | "G" | "T" | "Ki" | "Mi" | "Gi" | "Ti"
		)
}

/// Checks the BYOC pool flags. Every pool flag is optional: one left out is
/// not sent, and the cluster's operator keeps the pool's current value, or
/// 1 vCPU, 2Gi and 1 replica for a new pool.
pub fn pool_settings(
	cpu: Option<f64>,
	memory: Option<&str>,
	min_replicas: Option<u32>,
	max_replicas: Option<u32>,
	env: &[String],
	secret_env: &[String],
	pool_config: Option<&str>,
) -> Result<PoolSettings> {
	if let Some(cpu) = cpu
		&& (!cpu.is_finite() || cpu <= 0.0)
	{
		bail!("--cpu must be a positive number of vCPUs, got {cpu}");
	}
	if let Some(memory) = memory
		&& !quantity_like(memory)
	{
		bail!("--memory must be a Kubernetes quantity such as 512Mi or 2Gi, got {memory}");
	}
	let resources = match (cpu, memory) {
		(None, None) => None,
		(cpu, memory) => Some(ByocResources {
			cpu: cpu.map(|cpu| cpu.to_string()),
			memory: memory.map(str::to_string),
		}),
	};
	for (flag, replicas) in [
		("--min-replicas", min_replicas),
		("--max-replicas", max_replicas),
	] {
		if let Some(replicas) = replicas
			&& !(1..=MAX_REPLICAS).contains(&replicas)
		{
			bail!("{flag} must be between 1 and {MAX_REPLICAS}, got {replicas}");
		}
	}
	if let (Some(min), Some(max)) = (min_replicas, max_replicas)
		&& max < min
	{
		bail!("--max-replicas ({max}) must be at least --min-replicas ({min})");
	}

	let env = parse_pairs("--env", "KEY=VAL", env, |_| Ok(()))?;
	let secret_env =
		parse_pairs(
			"--secret-env",
			"KEY=SECRET/KEY",
			secret_env,
			|reference| match reference.split_once('/') {
				Some((secret, key))
					if !secret.is_empty() && !key.is_empty() && !key.contains('/') =>
				{
					Ok(())
				}
				_ => bail!("must name a Kubernetes Secret and key as SECRET/KEY, got {reference}"),
			},
		)?;
	if let (Some(env), Some(secret_env)) = (&env, &secret_env)
		&& let Some(name) = env.keys().find(|name| secret_env.contains_key(*name))
	{
		bail!("{name} is given by both --env and --secret-env; pass it once");
	}

	let pool_config = match pool_config {
		Some(raw) => {
			let value: Value =
				serde_json::from_str(raw).context("--pool-config must be a JSON object")?;
			let Value::Object(object) = value else {
				bail!("--pool-config must be a JSON object");
			};
			Some(object)
		}
		None => None,
	};

	Ok(PoolSettings {
		resources,
		min_replicas,
		max_replicas,
		env,
		secret_env,
		pool_config,
	})
}

/// Parses repeated `KEY=VALUE` flags. `None` when the flag is not given, so
/// the pool keeps its current variables.
fn parse_pairs(
	flag: &str,
	shape: &str,
	pairs: &[String],
	check_value: impl Fn(&str) -> Result<()>,
) -> Result<Option<BTreeMap<String, String>>> {
	if pairs.is_empty() {
		return Ok(None);
	}
	let mut map = BTreeMap::new();
	for pair in pairs {
		let Some((key, value)) = pair.split_once('=') else {
			bail!("{flag} must be {shape}, got {pair}");
		};
		validate_env_name(key).with_context(|| format!("{flag} {pair}"))?;
		check_value(value).with_context(|| format!("{flag} {key}"))?;
		if map.insert(key.to_string(), value.to_string()).is_some() {
			bail!("{flag} {key} is given more than once; pass each variable once");
		}
	}
	Ok(Some(map))
}

pub async fn deploy(cloud: &CloudClient, opts: ByocDeploy) -> Result<()> {
	let cluster = opts.cluster.as_str();
	let pool = opts.pool.as_str();

	// A build passed with --build is checked by the Cloud API when the pool is
	// applied, which refuses one that is not ready.
	let build_id = match opts.build {
		Some(build_id) => build_id,
		None => {
			let image = image_ref(cluster, opts.image, opts.tag);
			build_and_upload(
				cloud,
				cluster,
				&opts.build_context,
				&opts.dockerfile,
				&image,
			)
			.await?
		}
	};

	let settings = opts.pool_apply;
	let apply = ByocPoolApply {
		build_id: build_id.clone(),
		resources: settings.resources,
		min_replicas: settings.min_replicas,
		max_replicas: settings.max_replicas,
		env: settings.env,
		secret_env: settings.secret_env,
		pool_config: settings.pool_config,
	};
	tracing::info!(%cluster, %pool, %build_id, "applying pool");
	let command = v2::put_byoc_pool(cloud, cluster, pool, &apply).await?;
	let idx = command.idx;
	tracing::info!(%pool, command = idx, "queued pool apply for the cluster's operator");

	let code = wait_for_command(cloud, cluster, pool, command).await?;
	tracing::info!(%pool, %build_id, command = idx, %code, "pool applied");

	// The build id is the command's result; print it to stdout so it can be
	// captured by scripts and passed back as `--build`.
	println!("{build_id}");
	Ok(())
}

/// Builds and uploads a build without applying any pool, then prints its id.
pub async fn upload(cloud: &CloudClient, opts: ByocPrepare) -> Result<()> {
	let cluster = opts.cluster.as_str();
	let image = image_ref(cluster, opts.image, opts.tag);
	let build_id = build_and_upload(
		cloud,
		cluster,
		&opts.build_context,
		&opts.dockerfile,
		&image,
	)
	.await?;
	tracing::info!(
		%cluster,
		%build_id,
		"uploaded build without applying it; apply it with `rivet deploy --build`"
	);

	// The build id is the command's result; print it to stdout so it can be
	// captured by scripts and passed back as `--build`.
	println!("{build_id}");
	Ok(())
}

/// The image name recorded on a build, defaulting to the cluster name and the
/// git short SHA.
fn image_ref(cluster: &str, image: Option<String>, tag: Option<String>) -> String {
	format!(
		"{}:{}",
		image.as_deref().unwrap_or(cluster),
		tag.unwrap_or_else(default_image_tag)
	)
}

/// Builds the image, saves it to a private temporary tar and uploads it as a
/// ready build, returning its id. The tar is removed when this returns.
async fn build_and_upload(
	cloud: &CloudClient,
	cluster: &str,
	build_context: &Path,
	dockerfile: &Path,
	image: &str,
) -> Result<String> {
	tracing::info!("building Docker image");
	docker_build(build_context, dockerfile, image)?;
	let architecture = image_architecture(image)?;

	let dir = tempfile::tempdir().context("create temporary directory for the image tar")?;
	let tar = dir.path().join("image.tar");
	tracing::info!("saving Docker image");
	run_command(
		"docker",
		&["image", "save", image, "-o", &tar.to_string_lossy()],
		None,
	)?;

	push_tar(cloud, cluster, image, &architecture, &tar).await
}

/// Uploads a `docker image save` tar as a build of the cluster: creates the
/// build, PUTs the tar and completes it. Fails unless the build is ready.
pub async fn push_tar(
	cloud: &CloudClient,
	cluster: &str,
	image: &str,
	architecture: &str,
	tar: &Path,
) -> Result<String> {
	let digest_tar = tar.to_path_buf();
	let digest = tokio::task::spawn_blocking(move || digest(&digest_tar))
		.await
		.context("image digest task failed")??;
	if digest.content_length > MAX_BUILD_BYTES {
		bail!(
			"image tar is {} bytes, over the {MAX_BUILD_BYTES} byte build limit; reduce the image size",
			digest.content_length
		);
	}

	let created = v2::create_byoc_build(
		cloud,
		cluster,
		&NewByocBuild {
			image,
			architecture,
			content_length: digest.content_length,
			content_md5: &digest.content_md5,
			sha256: &digest.sha256,
		},
	)
	.await?;
	let build_id = created.build.id;
	tracing::info!(%build_id, content_length = digest.content_length, %architecture, "uploading build");
	v2::upload_byoc_build(cloud, &created.upload, tar, digest.content_length).await?;

	let completed = v2::complete_byoc_build(cloud, cluster, &build_id).await?;
	check_ready(&build_id, &completed)?;
	Ok(build_id)
}

/// Fails for a completed build that is not ready, saying why.
fn check_ready(build_id: &str, build: &ByocBuild) -> Result<()> {
	match build.status {
		ByocBuildStatus::Ready => Ok(()),
		ByocBuildStatus::Failed => bail!(
			"build {build_id} failed ({}); upload a new build",
			build.failure.as_deref().unwrap_or("no reason given")
		),
		ByocBuildStatus::Expired => {
			bail!("build {build_id} expired before its upload arrived; upload a new build")
		}
		ByocBuildStatus::Uploading => bail!(
			"build {build_id} is still waiting for its upload after the upload finished; upload a new build"
		),
	}
}

/// The image's architecture as Docker records it, which must be one the
/// Cloud API takes.
fn image_architecture(image: &str) -> Result<String> {
	let output = Command::new("docker")
		.args(["image", "inspect", "--format", "{{.Architecture}}", image])
		.output()
		.context("run docker image inspect; is Docker installed?")?;
	if !output.status.success() {
		bail!(
			"docker image inspect {image} failed with {}: {}",
			output.status,
			String::from_utf8_lossy(&output.stderr).trim()
		);
	}
	parse_architecture(&String::from_utf8_lossy(&output.stdout))
}

fn parse_architecture(raw: &str) -> Result<String> {
	let architecture = raw.trim();
	match architecture {
		"amd64" | "arm64" => Ok(architecture.to_string()),
		_ => {
			bail!("image architecture {architecture:?} is not supported; build for amd64 or arm64")
		}
	}
}

/// Returns the file's size, base64 MD5 and hex SHA-256 in one pass.
fn digest(path: &Path) -> Result<TarDigest> {
	let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
	let mut md5 = md5::Context::new();
	let mut sha256 = Sha256::new();
	let mut buffer = vec![0u8; 1024 * 1024];
	let mut content_length = 0u64;
	loop {
		let read = file
			.read(&mut buffer)
			.with_context(|| format!("read {}", path.display()))?;
		if read == 0 {
			break;
		}
		md5.consume(&buffer[..read]);
		sha256.update(&buffer[..read]);
		content_length += read as u64;
	}
	Ok(TarDigest {
		content_length,
		content_md5: STANDARD.encode(md5.compute().0),
		sha256: format!("{:x}", sha256.finalize()),
	})
}

/// Where a command is, for progress lines.
fn command_step(command: &ByocCommand) -> &'static str {
	match (command.status, &command.seen_at) {
		(Some(ByocCommandStatus::Completed), _) => "completed",
		(Some(ByocCommandStatus::Failed), _) => "failed",
		(Some(ByocCommandStatus::TimedOut), _) => "timed out",
		(None, Some(_)) => "the operator is running the command",
		(None, None) => "waiting for the cluster's operator to pick up the command",
	}
}

/// The command's result code, or `unknown` when the operator reported none.
fn result_code(command: &ByocCommand) -> String {
	command
		.result
		.as_ref()
		.and_then(|result| result.get("code"))
		.and_then(Value::as_str)
		.unwrap_or("unknown")
		.to_string()
}

/// The result of a reported command: its code when it completed, or an error
/// naming the code and the rest of the result when it failed. `None` while it
/// is not reported yet.
fn command_outcome(pool: &str, command: &ByocCommand) -> Result<Option<String>> {
	match command.status {
		None => Ok(None),
		Some(ByocCommandStatus::Completed) => Ok(Some(result_code(command))),
		Some(ByocCommandStatus::Failed) => {
			let code = result_code(command);
			let mut details = command.result.clone().unwrap_or_default();
			details.remove("code");
			let details = if details.is_empty() {
				String::new()
			} else {
				format!(" {}", Value::Object(details))
			};
			bail!(
				"applying pool {pool} failed: the operator reported {code}{details} for command {}",
				command.idx
			)
		}
		// The Cloud API's `timed_out` reason and time are for the Rivet team,
		// who are notified of the timeout; the user only learns that it timed out.
		Some(ByocCommandStatus::TimedOut) => {
			bail!("The pool rollout timed out. The Rivet team has been notified.")
		}
	}
}

/// Polls the queued command until the operator reports it and returns its
/// result code; a failed or timed-out command is an error.
async fn wait_for_command(
	cloud: &CloudClient,
	cluster: &str,
	pool: &str,
	queued: ByocCommand,
) -> Result<String> {
	let idx = queued.idx;
	let started = Instant::now();
	let deadline = started + COMMAND_SAFETY_CAP;
	let mut command = queued;
	let mut last_step: Option<&'static str> = None;
	let mut last_report = started;
	loop {
		if let Some(code) = command_outcome(pool, &command)? {
			return Ok(code);
		}

		let step = command_step(&command);
		let now = Instant::now();
		if last_step != Some(step) || now.duration_since(last_report) >= COMMAND_PROGRESS_INTERVAL {
			tracing::info!(%pool, command = idx, step, elapsed_s = started.elapsed().as_secs(), "pool apply in progress");
			last_step = Some(step);
			last_report = now;
		}

		if now + COMMAND_POLL_INTERVAL > deadline {
			bail!(
				"gave up after {} minutes waiting for the Cloud API to report command {idx} for pool {pool} as finished; check that the cluster's operator is running",
				COMMAND_SAFETY_CAP.as_secs() / 60
			);
		}
		sleep(COMMAND_POLL_INTERVAL).await;
		command = v2::get_byoc_command(cloud, cluster, idx).await?;
	}
}
