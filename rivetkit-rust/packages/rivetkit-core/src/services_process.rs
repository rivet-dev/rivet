use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, Url};
use rivet_error::RivetError;
use rivetkit_engine_process::download::{self, ReleaseBinary};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

use crate::time::sleep;

/// Dedicated pool used by the first-party services actor host.
pub(crate) const SERVICES_POOL_NAME: &str = "services";

/// Services release this RivetKit build runs in local development. It is
/// downloaded on demand from `releases.rivet.dev/services/{version}/services/`.
/// Bump it whenever RivetKit should run a newer Services release.
pub(crate) const SERVICES_VERSION: &str = "0.2.3";
const SERVICES_VERSION_ENV: &str = "RIVETKIT_SERVICES_VERSION";
const SERVICES_BINARY_ENV: &str = "RIVET_SERVICES_BINARY";
const SERVICES_RELEASES_ENDPOINT_ENV: &str = "RIVETKIT_SERVICES_RELEASES_ENDPOINT";
const SERVICES_DOWNLOAD_ALTERNATIVE: &str = "set RIVET_RUN_SERVICES=0 to run without Services.";

const READINESS_MAX_ATTEMPTS: usize = 60;
const READINESS_RETRY_DELAY: Duration = Duration::from_millis(500);
// The child is itself a RivetKit registry. SIGTERM starts its normal envoy and
// actor drain, so allow the same 30-minute fallback used by serverful RivetKit
// before escalating to SIGKILL.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Debug)]
pub(crate) struct ServicesProcessConfig {
	pub binary_path: Option<PathBuf>,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub engine_protocol_version: u16,
	/// Whether a missing binary may be downloaded from releases.rivet.dev.
	pub auto_download: bool,
}

#[derive(Debug)]
pub(crate) struct ServicesProcessManager {
	child: Child,
}

#[derive(Debug, Deserialize)]
struct EnvoysResponse {
	envoys: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicesVersionOutput {
	name: String,
	version: String,
	rivetkit_version: String,
	protocol_version: u16,
}

#[derive(RivetError, Debug, Serialize)]
#[error("services")]
enum ServicesProcessError {
	#[error(
		"binary_unavailable",
		"Services binary is unavailable.",
		"Rivet Services {version} is not cached and downloading it is turned off by RIVETKIT_ENGINE_AUTO_DOWNLOAD or ServeConfig::engine_auto_download. Set RIVET_SERVICES_BINARY to a rivet-services binary, turn downloads back on with RIVETKIT_ENGINE_AUTO_DOWNLOAD=1 or engine_auto_download, or set RIVET_RUN_SERVICES=0 to disable Services."
	)]
	BinaryUnavailable { version: String },

	#[error(
		"binary_not_found",
		"Services binary was not found.",
		"Services binary was not found at '{path}'. Point RIVET_SERVICES_BINARY at an existing rivet-services binary, unset it so RivetKit downloads Services, or set RIVET_RUN_SERVICES=0 to disable Services."
	)]
	BinaryNotFound { path: String },

	#[error(
		"metadata_failed",
		"Services compatibility could not be verified.",
		"Services compatibility could not be verified: {reason}"
	)]
	MetadataFailed { reason: String },

	#[error(
		"protocol_mismatch",
		"Services is newer than the local Engine protocol.",
		"Services uses Envoy protocol {services_protocol_version}, but this RivetKit Engine supports {engine_protocol_version}. Upgrade RivetKit or point RIVET_SERVICES_BINARY at an older rivet-services binary."
	)]
	ProtocolMismatch {
		services_protocol_version: u16,
		engine_protocol_version: u16,
	},

	#[error(
		"start_failed",
		"Services failed to start.",
		"Services failed to start: {reason}"
	)]
	StartFailed { reason: String },

	#[error(
		"readiness_failed",
		"Services did not become ready.",
		"Services did not register in pool '{pool_name}': {reason}"
	)]
	ReadinessFailed { pool_name: String, reason: String },
}

impl ServicesProcessManager {
	pub(crate) async fn start(config: ServicesProcessConfig) -> Result<Self> {
		let binary_path = resolve_services_binary(&config).await?;
		let binary_path = binary_path.as_path();
		if !binary_path.exists() {
			return Err(ServicesProcessError::BinaryNotFound {
				path: binary_path.display().to_string(),
			}
			.build());
		}

		validate_binary_compatibility(binary_path, &config).await?;

		let mut command = Command::new(binary_path);
		command
			.arg("start")
			.env("RIVET_ENDPOINT", &config.endpoint)
			.env("RIVET__AUTH__ADMIN_TOKEN", "default")
			.env("RIVET_NAMESPACE", &config.namespace)
			.env("RIVET_POOL_NAME", &config.pool_name)
			.env("RIVETKIT_ENGINE_SPAWN", "never")
			// Prevent the child RivetKit registry from recursively starting another
			// Services process.
			.env("RIVET_RUN_SERVICES", "0")
			.stdin(Stdio::null())
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.kill_on_drop(true);
		if let Some(token) = &config.token {
			command.env("RIVET_TOKEN", token);
		} else {
			command.env_remove("RIVET_TOKEN");
		}

		let mut child = command.spawn().map_err(|error| {
			ServicesProcessError::StartFailed {
				reason: format!("could not spawn `{}`: {error}", binary_path.display()),
			}
			.build()
		})?;
		wait_for_readiness(
			&mut child,
			&config,
			READINESS_MAX_ATTEMPTS,
			READINESS_RETRY_DELAY,
		)
		.await?;

		tracing::info!(
			pid = child.id(),
			path = %binary_path.display(),
			endpoint = %config.endpoint,
			namespace = %config.namespace,
			pool_name = %config.pool_name,
			"Services process is ready"
		);
		Ok(Self { child })
	}

	pub(crate) async fn shutdown(mut self) {
		if self.child.try_wait().ok().flatten().is_some() {
			return;
		}

		#[cfg(unix)]
		let signaled = self.child.id().is_some_and(|pid| {
			use nix::sys::signal::{Signal, kill};
			use nix::unistd::Pid;

			kill(Pid::from_raw(pid as i32), Signal::SIGTERM).is_ok()
		});
		#[cfg(not(unix))]
		let signaled = false;

		if !signaled && self.child.start_kill().is_err() {
			return;
		}

		if tokio::time::timeout(SHUTDOWN_TIMEOUT, self.child.wait())
			.await
			.is_err()
		{
			tracing::warn!(pid = self.child.id(), "Services did not stop; killing it");
			let _ = self.child.start_kill();
			let _ = self.child.wait().await;
		}
	}
}

/// Resolves the Services binary: the configured path (which already includes
/// `RIVET_SERVICES_BINARY`), then the pinned release from the per-user cache,
/// downloading it on first use.
async fn resolve_services_binary(config: &ServicesProcessConfig) -> Result<PathBuf> {
	if let Some(path) = &config.binary_path {
		return Ok(path.clone());
	}

	let release = services_release_binary(
		&std::env::var(SERVICES_VERSION_ENV).unwrap_or_else(|_| SERVICES_VERSION.to_owned()),
		&std::env::var(SERVICES_RELEASES_ENDPOINT_ENV)
			.unwrap_or_else(|_| download::RELEASES_ENDPOINT.to_owned()),
	)?;
	let cache_root = download::binary_cache_root()?;
	// `auto_download` is already false when RIVET_DISABLE_BINARY_DOWNLOAD is
	// set, which the downloader reports with its own error.
	if !config.auto_download
		&& !download::download_disabled_by_env()
		&& !release.is_cached(&cache_root)
	{
		return Err(ServicesProcessError::BinaryUnavailable {
			version: release.version.clone(),
		}
		.build());
	}
	download::ensure_release_binary(
		&release,
		&cache_root,
		config.auto_download && !download::download_disabled_by_env(),
		"RivetKit runs Services next to the local Rivet Engine",
	)
	.await
}

/// Whether to run without Services because no Services binary is published for
/// this platform, no binary was provided, and Services was not explicitly
/// requested. Logs a warning with the ways to enable or silence it.
pub(crate) fn skip_unsupported_services(config: &ServicesProcessConfig, required: bool) -> bool {
	if !should_skip_services(
		required,
		services_artifact().is_some(),
		config.binary_path.is_some(),
	) {
		return false;
	}
	tracing::warn!(
		os = std::env::consts::OS,
		arch = std::env::consts::ARCH,
		musl = cfg!(target_env = "musl"),
		hint = "set RIVET_SERVICES_BINARY to a rivet-services binary built for this platform, or set RIVET_RUN_SERVICES=0 to hide this warning",
		"Rivet Services is not published for this platform, so RivetKit is running without Services"
	);
	true
}

fn should_skip_services(required: bool, platform_supported: bool, has_binary: bool) -> bool {
	!required && !platform_supported && !has_binary
}

/// Describes the pinned Services release binary for this host.
fn services_release_binary(version: &str, releases_endpoint: &str) -> Result<ReleaseBinary> {
	let artifact = services_artifact().ok_or_else(|| {
		download::unsupported_platform(
			"Rivet Services",
			SERVICES_BINARY_ENV,
			SERVICES_DOWNLOAD_ALTERNATIVE,
		)
	})?;
	download::validate_pinned_version("Rivet Services", version)?;
	let platform = artifact
		.trim_start_matches("rivet-services-")
		.trim_end_matches(".exe");
	Ok(ReleaseBinary {
		display_name: "Rivet Services",
		cache_name: "services",
		version: version.to_owned(),
		platform: platform.to_owned(),
		base_url: format!(
			"{}/services/{version}/services",
			releases_endpoint.trim_end_matches('/')
		),
		artifact: artifact.to_owned(),
		file_name: if cfg!(windows) {
			"rivet-services.exe".to_owned()
		} else {
			"rivet-services".to_owned()
		},
		override_env: SERVICES_BINARY_ENV,
		alternative: SERVICES_DOWNLOAD_ALTERNATIVE,
	})
}

/// Release artifact name of the Services binary for this host, or `None` when
/// no release binary is published for it. Services ships glibc builds only,
/// so musl hosts are unsupported.
fn services_artifact() -> Option<&'static str> {
	if cfg!(target_env = "musl") {
		return None;
	}
	match (std::env::consts::OS, std::env::consts::ARCH) {
		("linux", "x86_64") => Some("rivet-services-linux-x64-gnu"),
		("linux", "aarch64") => Some("rivet-services-linux-arm64-gnu"),
		("macos", "x86_64") => Some("rivet-services-darwin-x64"),
		("macos", "aarch64") => Some("rivet-services-darwin-arm64"),
		("windows", "x86_64") => Some("rivet-services-win32-x64.exe"),
		_ => None,
	}
}

async fn validate_binary_compatibility(
	binary_path: &Path,
	config: &ServicesProcessConfig,
) -> Result<()> {
	let output = run_metadata_command(binary_path, "--version").await?;
	let output: ServicesVersionOutput = serde_json::from_str(&output)
		.map_err(|error| metadata_error(format!("invalid --version output: {error}")))?;
	if output.name != "rivet-services" {
		return Err(metadata_error(format!(
			"unexpected binary name `{}` in --version output",
			output.name
		)));
	}
	tracing::debug!(
		services_version = %output.version,
		services_rivetkit_version = %output.rivetkit_version,
		services_protocol_version = output.protocol_version,
		engine_protocol_version = config.engine_protocol_version,
		"validated Services binary metadata"
	);
	// This protocol comparison is the compatibility safeguard. The Services
	// version is diagnostic only, so bump `SERVICES_VERSION` to a release whose
	// Envoy protocol this RivetKit supports.
	validate_protocol_version(output.protocol_version, config.engine_protocol_version)
}

fn validate_protocol_version(services_version: u16, engine_version: u16) -> Result<()> {
	if services_version > engine_version {
		return Err(ServicesProcessError::ProtocolMismatch {
			services_protocol_version: services_version,
			engine_protocol_version: engine_version,
		}
		.build());
	}
	Ok(())
}

async fn run_metadata_command(binary_path: &Path, argument: &str) -> Result<String> {
	let output = Command::new(binary_path)
		.arg(argument)
		.stdin(Stdio::null())
		.kill_on_drop(true)
		.output()
		.await
		.map_err(|error| {
			metadata_error(format!(
				"could not run `{}` {argument}: {error}",
				binary_path.display()
			))
		})?;
	if !output.status.success() {
		return Err(metadata_error(format!(
			"`{}` {argument} exited with {}: {}",
			binary_path.display(),
			output.status,
			String::from_utf8_lossy(&output.stderr).trim()
		)));
	}

	String::from_utf8(output.stdout)
		.map(|output| output.trim().to_owned())
		.map_err(|error| metadata_error(format!("{argument} returned invalid UTF-8: {error}")))
}

fn metadata_error(reason: impl Into<String>) -> anyhow::Error {
	ServicesProcessError::MetadataFailed {
		reason: reason.into(),
	}
	.build()
}

async fn wait_for_readiness(
	child: &mut Child,
	config: &ServicesProcessConfig,
	max_attempts: usize,
	retry_delay: Duration,
) -> Result<()> {
	let client = Client::builder()
		.build()
		.context("build Services readiness client")?;
	let mut url = Url::parse(&config.endpoint)
		.with_context(|| format!("parse Engine endpoint `{}`", config.endpoint))?;
	url.set_path("/envoys");
	url.set_query(None);
	url.query_pairs_mut()
		.append_pair("namespace", &config.namespace)
		.append_pair("name", &config.pool_name);

	let max_attempts = max_attempts.max(1);
	let mut last_reason = "the Engine has not listed the Services Envoy".to_owned();
	for attempt in 1..=max_attempts {
		if let Some(status) = child.try_wait().map_err(|error| {
			ServicesProcessError::StartFailed {
				reason: format!("could not inspect child process: {error}"),
			}
			.build()
		})? {
			return Err(ServicesProcessError::StartFailed {
				reason: format!("process exited with status {status}"),
			}
			.build());
		}

		let mut request = client.get(url.clone());
		if let Some(token) = &config.token {
			request = request.bearer_auth(token);
		}
		match request.send().await {
			Ok(response) if response.status().is_success() => {
				match response.json::<EnvoysResponse>().await {
					Ok(response) if !response.envoys.is_empty() => return Ok(()),
					Ok(_) => {
						last_reason = "the Engine returned no matching envoys".to_owned();
					}
					Err(error) => last_reason = format!("invalid Engine response: {error}"),
				}
			}
			Ok(response) => {
				let status = response.status();
				let body = response.text().await.unwrap_or_default();
				last_reason = format!("Engine returned {status}: {body}");
			}
			Err(error) => last_reason = error.to_string(),
		}

		if attempt < max_attempts {
			sleep(retry_delay).await;
		}
	}

	Err(ServicesProcessError::ReadinessFailed {
		pool_name: config.pool_name.clone(),
		reason: last_reason,
	}
	.build())
}

#[cfg(test)]
#[path = "../tests/services_process.rs"]
mod tests;
