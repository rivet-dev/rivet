use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::Url;
use serde::Deserialize;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

use crate::error::EngineProcessError;

const ENGINE_RUNTIME: &str = "engine";
const RIVETKIT_RUNTIME: &str = "rivetkit";

#[derive(Debug, Deserialize)]
struct EngineHealthResponse {
	status: Option<String>,
	runtime: Option<String>,
	version: Option<String>,
}

/// Manages the rivet-engine subprocess.
///
/// The engine is intentionally orphaned: dropping the manager (or having the
/// host process exit) must NOT terminate the engine. This lets a dev-server
/// restart of the rivetkit host reattach to the same long-lived engine and
/// keep all in-flight actor state. To honor that contract:
///
/// - `Command::kill_on_drop` is left at its default (false) so the tokio
///   `Child` does not send SIGKILL on drop.
/// - Stdout and stderr are routed to log files at spawn time so the engine's
///   write fds remain valid after the host's pipes close.
/// - On startup we probe the configured endpoint and reuse a healthy engine
///   instead of spawning a duplicate.
///
/// When we spawn the engine, `watcher` holds a tokio task that owns the
/// `Child` and awaits `child.wait()` so we get a log line if the engine dies
/// while rivetkit is still running. On Drop we abort the watcher; aborting
/// drops the `Child` without killing it (kill_on_drop=false), so the engine
/// stays running and gets reparented to init when rivetkit exits.
///
/// `watcher` is `None` when we attached to an already-running engine.
#[derive(Debug)]
pub(crate) struct EngineProcessManager {
	watcher: Option<JoinHandle<()>>,
}

impl EngineProcessManager {
	pub(crate) async fn start(binary_path: &Path, endpoint: &str) -> Result<Self> {
		if let Some(health) = probe_existing_engine(endpoint).await? {
			tracing::info!(
				endpoint = %endpoint,
				status = ?health.status,
				runtime = ?health.runtime,
				version = ?health.version,
				"reusing already-running engine process"
			);
			return Ok(Self { watcher: None });
		}

		if !binary_path.exists() {
			return Err(EngineProcessError::BinaryNotFound {
				path: binary_path.display().to_string(),
			}
			.build());
		}

		let endpoint_url =
			Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
		let guard_host = endpoint_url
			.host_str()
			.ok_or_else(|| invalid_endpoint(endpoint, "missing host"))?
			.to_owned();
		let guard_port = endpoint_url
			.port_or_known_default()
			.ok_or_else(|| invalid_endpoint(endpoint, "missing port"))?;
		let api_peer_port = guard_port
			.checked_add(1)
			.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?;
		let metrics_port = guard_port
			.checked_add(10)
			.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?;

		let storage_root = storage_root()?;
		let var_dir = storage_root.join("var");
		let db_path = var_dir.join("engine").join("db");
		let logs_dir = var_dir.join("logs").join("rivet-engine");
		ensure_dir(&db_path).context("create engine db directory")?;
		ensure_dir(&logs_dir).context("create engine logs directory")?;

		let timestamp = log_timestamp();
		let stdout_log_path = logs_dir.join(format!("engine-{timestamp}-stdout.log"));
		let stderr_log_path = logs_dir.join(format!("engine-{timestamp}-stderr.log"));
		let stdout_file = open_log_file(&stdout_log_path)
			.with_context(|| format!("open engine stdout log `{}`", stdout_log_path.display()))?;
		let stderr_file = open_log_file(&stderr_log_path)
			.with_context(|| format!("open engine stderr log `{}`", stderr_log_path.display()))?;

		let mut command = Command::new(binary_path);
		command
			.arg("start")
			.env("RIVET__GUARD__HOST", &guard_host)
			.env("RIVET__GUARD__PORT", guard_port.to_string())
			.env("RIVET__API_PEER__HOST", &guard_host)
			.env("RIVET__API_PEER__PORT", api_peer_port.to_string())
			.env("RIVET__METRICS__HOST", &guard_host)
			.env("RIVET__METRICS__PORT", metrics_port.to_string())
			.env("RIVET__FILE_SYSTEM__PATH", &db_path)
			.stdin(Stdio::null())
			.stdout(Stdio::from(stdout_file))
			.stderr(Stdio::from(stderr_file));

		// Put the engine in its own process group so terminal signals
		// (Ctrl+C, Ctrl+Z, SIGHUP on terminal close) targeting our foreground
		// process group do not reach the engine. Combined with no-kill-on-drop
		// and file-fd stdio, this gives the engine a real "intentional orphan"
		// lifetime that survives the host being killed for any reason.
		#[cfg(unix)]
		command.process_group(0);

		let mut child = command
			.spawn()
			.with_context(|| format!("spawn engine binary `{}`", binary_path.display()))?;
		let pid = child
			.id()
			.ok_or_else(|| EngineProcessError::MissingPid.build())?;

		tracing::info!(
			pid,
			path = %binary_path.display(),
			endpoint = %endpoint,
			db_path = %db_path.display(),
			"spawned engine process (intentionally orphaned, will outlive this process)"
		);
		tracing::info!(
			stdout_log = %stdout_log_path.display(),
			stderr_log = %stderr_log_path.display(),
			"engine stdout/stderr piped to log files"
		);

		let health_url = engine_health_url(endpoint);
		let health = match wait_for_engine_health(&health_url).await {
			Ok(health) => health,
			Err(error) => {
				let error = match child.try_wait() {
					Ok(Some(status)) => error.context(format!(
						"engine process exited before becoming healthy with status {status}"
					)),
					Ok(None) => error,
					Err(wait_error) => error.context(format!(
						"failed to inspect engine process status: {wait_error:#}"
					)),
				};
				if let Err(cleanup_error) = terminate_failed_spawn(&mut child).await {
					tracing::warn!(
						?cleanup_error,
						"failed to terminate engine process that never became healthy"
					);
				}
				return Err(error);
			}
		};

		tracing::info!(
			pid,
			status = ?health.status,
			runtime = ?health.runtime,
			version = ?health.version,
			"engine process is healthy"
		);

		Ok(Self {
			watcher: Some(spawn_engine_watcher(child, pid)),
		})
	}
}

impl Drop for EngineProcessManager {
	fn drop(&mut self) {
		if let Some(handle) = self.watcher.take() {
			// Aborting drops the `Child` owned by the task. With
			// `kill_on_drop=false`, dropping the `Child` does NOT signal the
			// engine, so the engine survives and gets reparented to init.
			// We give up our crash-detection log line here, but if we are
			// being dropped the rivetkit host is shutting down anyway.
			handle.abort();
			tracing::debug!(
				"aborted engine watcher; engine continues running (intentional orphan)"
			);
		}
	}
}

/// Spawns a background task that owns the `Child` and awaits `wait()` so we
/// log a clear message if the engine dies while rivetkit is still up. Taking
/// the `Child` into the task also reaps it via `waitpid` on exit, so a
/// crashed engine never lingers as a zombie in our process table.
fn spawn_engine_watcher(mut child: Child, pid: u32) -> JoinHandle<()> {
	tokio::spawn(async move {
		match child.wait().await {
			Ok(status) if status.success() => {
				tracing::warn!(
					pid,
					?status,
					"engine process exited cleanly while rivetkit was still running; \
					 rivetkit expected the engine to outlive it"
				);
			}
			Ok(status) => {
				tracing::error!(
					pid,
					?status,
					"engine process crashed while rivetkit was still running"
				);
			}
			Err(error) => {
				tracing::error!(
					pid,
					?error,
					"failed to wait on engine process; cannot detect crashes"
				);
			}
		}
	})
}

/// Probes the configured endpoint for an already-running, healthy engine.
///
/// Returns `Ok(Some(health))` if the endpoint is serving a `runtime: "engine"`
/// health response that we can reattach to. Returns `Ok(None)` if the port is
/// free. Returns `Err(...)` if the port is occupied by a non-engine process
/// (for example a stale rivetkit) which would conflict with a fresh spawn.
async fn probe_existing_engine(endpoint: &str) -> Result<Option<EngineHealthResponse>> {
	let health_url = engine_health_url(endpoint);
	let client = rivet_pools::reqwest::client()
		.await
		.context("build reqwest client for engine probe")?;

	let response = match client
		.get(&health_url)
		.timeout(Duration::from_secs(1))
		.send()
		.await
	{
		Ok(response) => response,
		Err(_) => return Ok(None),
	};

	if !response.status().is_success() {
		return Ok(None);
	}

	let health = response
		.json::<EngineHealthResponse>()
		.await
		.context("decode existing engine health response")?;

	match health.runtime.as_deref() {
		Some(ENGINE_RUNTIME) => Ok(Some(health)),
		Some(RIVETKIT_RUNTIME) => Err(EngineProcessError::PortOccupied {
			endpoint: endpoint.to_owned(),
			runtime: RIVETKIT_RUNTIME.to_owned(),
		}
		.build()),
		Some(other) => Err(EngineProcessError::PortOccupied {
			endpoint: endpoint.to_owned(),
			runtime: other.to_owned(),
		}
		.build()),
		None => Err(EngineProcessError::PortOccupied {
			endpoint: endpoint.to_owned(),
			runtime: "unknown".to_owned(),
		}
		.build()),
	}
}

fn engine_health_url(endpoint: &str) -> String {
	format!("{}/health", endpoint.trim_end_matches('/'))
}

fn storage_root() -> Result<PathBuf> {
	if let Ok(path) = std::env::var("RIVETKIT_STORAGE_PATH") {
		return Ok(PathBuf::from(path).join(".rivetkit"));
	}
	let home = std::env::var("HOME")
		.map(PathBuf::from)
		.or_else(|_| std::env::current_dir())
		.context("locate home directory for engine storage path")?;
	Ok(home.join(".rivetkit"))
}

fn ensure_dir(path: &Path) -> Result<()> {
	std::fs::create_dir_all(path).with_context(|| format!("create directory `{}`", path.display()))
}

fn open_log_file(path: &Path) -> Result<std::fs::File> {
	std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(path)
		.with_context(|| format!("open log file `{}`", path.display()))
}

fn log_timestamp() -> String {
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default();
	format!("{}", now.as_secs())
}

async fn wait_for_engine_health(health_url: &str) -> Result<EngineHealthResponse> {
	const HEALTH_MAX_WAIT: Duration = Duration::from_secs(10);
	const HEALTH_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
	const HEALTH_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
	const HEALTH_MAX_BACKOFF: Duration = Duration::from_secs(1);

	let client = rivet_pools::reqwest::client()
		.await
		.context("build reqwest client for engine health check")?;
	let deadline = Instant::now() + HEALTH_MAX_WAIT;
	let mut attempt = 0u32;
	let mut backoff = HEALTH_INITIAL_BACKOFF;

	loop {
		attempt += 1;

		let last_error = match client
			.get(health_url)
			.timeout(HEALTH_REQUEST_TIMEOUT)
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => {
				let health = response
					.json::<EngineHealthResponse>()
					.await
					.context("decode engine health response")?;
				return Ok(health);
			}
			Ok(response) => format!("unexpected status {}", response.status()),
			Err(error) => error.to_string(),
		};

		if Instant::now() >= deadline {
			return Err(EngineProcessError::HealthCheckFailed {
				attempts: attempt,
				reason: last_error,
			}
			.build());
		}

		tokio::time::sleep(backoff).await;
		backoff = std::cmp::min(backoff * 2, HEALTH_MAX_BACKOFF);
	}
}

/// Cleanup path for a spawn that never reached `healthy`. We *do* kill here
/// because the half-started engine has no useful state to preserve and
/// leaving it running would conflict with a retry. This is the only place
/// allowed to terminate the engine.
async fn terminate_failed_spawn(child: &mut Child) -> Result<()> {
	const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

	if child
		.try_wait()
		.context("check engine process status")?
		.is_some()
	{
		return Ok(());
	}

	child
		.start_kill()
		.context("kill half-started engine process")?;
	match tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
		Ok(result) => {
			let status = result.context("wait for half-started engine to exit")?;
			tracing::info!(?status, "half-started engine process exited");
			Ok(())
		}
		Err(_) => {
			tracing::warn!("half-started engine process did not exit within timeout");
			Ok(())
		}
	}
}

fn invalid_endpoint(endpoint: &str, reason: &str) -> anyhow::Error {
	EngineProcessError::InvalidEndpoint {
		endpoint: endpoint.to_owned(),
		reason: reason.to_owned(),
	}
	.build()
}
