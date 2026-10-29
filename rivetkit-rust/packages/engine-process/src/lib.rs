use std::process::Stdio;
use std::time::{Duration, Instant};
use std::{
	env,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

mod error;

pub use error::EngineProcessError;

const ENGINE_RUNTIME: &str = "engine";
/// Engine architecture this RivetKit requires. v2 engines report the same runtime and
/// version but no architecture, so the field alone tells them apart.
const ENGINE_ARCHITECTURE: &str = "v3";
const RIVETKIT_RUNTIME: &str = "rivetkit";
const ENGINE_VERSION_ENV: &str = "RIVETKIT_ENGINE_VERSION";
const RELEASES_ENDPOINT_ENV: &str = "RIVETKIT_ENGINE_RELEASES_ENDPOINT";
const RELEASES_ENDPOINT: &str = "https://releases.rivet.dev";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
struct EngineHealthResponse {
	status: Option<String>,
	runtime: Option<String>,
	version: Option<String>,
	architecture: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EngineResolverConfig {
	pub endpoint: String,
	pub explicit_binary_path: Option<PathBuf>,
	pub bind_host: Option<String>,
	pub bind_port: Option<u16>,
	pub public_url: Option<String>,
	pub auto_download: bool,
	pub version: String,
	pub releases_endpoint: String,
}

impl EngineResolverConfig {
	pub fn from_parts(
		endpoint: &str,
		explicit_binary_path: Option<PathBuf>,
		bind_host: Option<String>,
		bind_port: Option<u16>,
		auto_download: bool,
	) -> Self {
		Self {
			endpoint: endpoint.to_owned(),
			explicit_binary_path,
			bind_host,
			bind_port,
			public_url: None,
			auto_download,
			version: std::env::var(ENGINE_VERSION_ENV)
				.unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned()),
			releases_endpoint: std::env::var(RELEASES_ENDPOINT_ENV)
				.unwrap_or_else(|_| RELEASES_ENDPOINT.to_owned()),
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedEngine {
	Existing,
	Binary(PathBuf),
}

/// Effective runtime parameters of a spawned engine, stamped to disk so a later
/// process that reattaches can tell how the running engine is configured without
/// probing it over the network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineRuntimeStamp {
	pub pid: u32,
	pub endpoint: String,
	pub bind_host: String,
	pub public_url: Option<String>,
}

impl EngineRuntimeStamp {
	/// True if the engine bound a loopback address only, so peers on other hosts
	/// (for example a container reaching the host) cannot connect to it.
	pub fn binds_loopback_only(&self) -> bool {
		if self.bind_host == "localhost" {
			return true;
		}
		self.bind_host
			.parse::<std::net::IpAddr>()
			.map(|ip| ip.is_loopback())
			.unwrap_or(false)
	}
}

/// How `start_or_reuse` satisfied the request.
#[derive(Debug)]
pub enum EngineStartup {
	/// A new engine was spawned with the requested configuration.
	Spawned,
	/// An already-running engine was reused. `stamp` is its recorded runtime
	/// parameters, or `None` when no live stamp was found (an engine from an
	/// older CLI, started by hand, or a stale stamp whose process is gone).
	Reused { stamp: Option<EngineRuntimeStamp> },
}

/// Manages the rivet-engine subprocess.
///
/// A spawned engine belongs to this host and is stopped on shutdown. An engine
/// found at the endpoint is only reused, never stopped by this manager.
#[derive(Debug)]
pub struct EngineProcessManager {
	watcher: Option<JoinHandle<()>>,
	stop: Option<oneshot::Sender<()>>,
	startup: EngineStartup,
}

impl EngineProcessManager {
	pub async fn start_or_reuse(config: EngineResolverConfig) -> Result<Self> {
		// Remove a dead process's record before deciding whether to attach or spawn.
		// The endpoint probe below is still authoritative for a live engine.
		let _ = read_engine_stamp();
		let resolved = resolve_engine_binary(&config).await?;
		Self::start_resolved(resolved, &config).await
	}

	/// How the engine was obtained: freshly spawned or reused.
	pub fn startup(&self) -> &EngineStartup {
		&self.startup
	}

	pub async fn shutdown(mut self) {
		if let Some(stop) = self.stop.take() {
			let _ = stop.send(());
		}
		if let Some(watcher) = self.watcher.take() {
			let _ = watcher.await;
		}
	}

	async fn start_resolved(
		resolved: ResolvedEngine,
		config: &EngineResolverConfig,
	) -> Result<Self> {
		let endpoint = &config.endpoint;
		if matches!(resolved, ResolvedEngine::Existing) {
			tracing::info!(
				endpoint = %endpoint,
				"reusing already-running engine process"
			);
			return Ok(Self {
				watcher: None,
				stop: None,
				startup: EngineStartup::Reused {
					stamp: read_engine_stamp(),
				},
			});
		}

		let ResolvedEngine::Binary(binary_path) = resolved else {
			unreachable!("existing engine handled above");
		};
		let mut health = probe_existing_engine(endpoint).await?;
		if health.is_none() {
			if let Some(stamp) = read_engine_stamp().filter(|stamp| stamp.endpoint == *endpoint) {
				// Shutdown closes the listener before releasing the storage locks.
				// A watcher may restart its app while that Engine is still draining.
				wait_for_engine_exit(stamp.pid, Duration::from_secs(20)).await?;
				// Another app may have completed startup while we waited.
				health = probe_existing_engine(endpoint).await?;
			}
		}
		if let Some(health) = health {
			tracing::info!(
				endpoint = %endpoint,
				status = ?health.status,
				runtime = ?health.runtime,
				version = ?health.version,
				"reusing already-running engine process"
			);
			return Ok(Self {
				watcher: None,
				stop: None,
				startup: EngineStartup::Reused {
					stamp: read_engine_stamp(),
				},
			});
		}

		if !binary_path.exists() {
			return Err(EngineProcessError::BinaryNotFound {
				path: binary_path.display().to_string(),
			}
			.build());
		}

		ensure_compatible_binary(&binary_path).await?;

		let env = engine_env(config)?;
		let config_path = write_engine_config(config)?;
		let data_path = engine_data_path()?;
		let logs_dir = storage_root()?
			.join("var")
			.join("logs")
			.join("rivet-engine");
		ensure_dir(&logs_dir).context("create engine logs directory")?;
		ensure_dir(&engine_log_directory()?).context("create engine log shard directory")?;

		let timestamp = log_timestamp();
		let stdout_log_path = logs_dir.join(format!("engine-{timestamp}-stdout.log"));
		let stderr_log_path = logs_dir.join(format!("engine-{timestamp}-stderr.log"));
		let stdout_file = open_log_file(&stdout_log_path)
			.with_context(|| format!("open engine stdout log `{}`", stdout_log_path.display()))?;
		let stderr_file = open_log_file(&stderr_log_path)
			.with_context(|| format!("open engine stderr log `{}`", stderr_log_path.display()))?;

		let mut command = Command::new(&binary_path);
		command.arg("--config").arg(&config_path);
		for (key, value) in &env {
			command.env(key, value);
		}
		command
			.stdin(Stdio::null())
			.stdout(Stdio::from(stdout_file))
			.stderr(Stdio::from(stderr_file));

		// Do not send terminal signals to the engine twice: the host drains its
		// actors first, then asks the watcher to stop the engine.
		#[cfg(unix)]
		command.process_group(0);
		// A file watcher can terminate its child without running Rust destructors.
		// Linux still sends the engine SIGTERM when that host process disappears.
		#[cfg(target_os = "linux")]
		unsafe {
			use std::os::unix::process::CommandExt;
			let parent_pid = libc::getpid();
			command.as_std_mut().pre_exec(move || {
				if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
					return Err(std::io::Error::last_os_error());
				}
				if libc::getppid() != parent_pid {
					libc::raise(libc::SIGTERM);
				}
				Ok(())
			});
		}

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
			data_path = %data_path.display(),
			"spawned engine process"
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

		// An explicit or cached binary may be an older engine. Never leave one running.
		let ready = match ensure_compatible_engine(endpoint, &health) {
			Ok(()) => wait_for_engine_ready(endpoint).await,
			Err(error) => Err(error),
		};
		if let Err(error) = ready {
			if let Err(cleanup_error) = terminate_failed_spawn(&mut child).await {
				tracing::warn!(
					?cleanup_error,
					"failed to terminate engine process that never became ready"
				);
			}
			return Err(error);
		}
		tracing::info!(pid, "engine process is ready");

		// Record the effective runtime parameters so a later reattaching process
		// can tell how this engine is bound. Best-effort: a failed write only
		// costs that later process its reuse diagnostics.
		let stamp = EngineRuntimeStamp {
			pid,
			endpoint: endpoint.clone(),
			bind_host: resolve_bind_host(config)?,
			public_url: config.public_url.clone(),
		};
		if let Err(error) = write_engine_stamp(&stamp) {
			tracing::warn!(?error, "failed to write engine runtime stamp");
		}

		let (watcher, stop) = spawn_engine_watcher(child, pid);
		Ok(Self {
			watcher: Some(watcher),
			stop: Some(stop),
			startup: EngineStartup::Spawned,
		})
	}
}

/// Waits without signalling a previous Engine that may still own storage locks.
async fn wait_for_engine_exit(pid: u32, timeout: Duration) -> Result<()> {
	tracing::info!(
		pid,
		"waiting for the previous local engine to finish stopping"
	);
	tokio::time::timeout(timeout, async {
		while pid_is_alive(pid) {
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	})
	.await
	.with_context(|| {
		format!(
			"previous local Engine process {pid} is still stopping; refusing to start another Engine against its storage"
		)
	})?;
	Ok(())
}

/// How long the `--runtime-info` probe may run before the binary is treated as incompatible.
// A freshly linked native binary can spend several seconds in the OS loader
// before reaching main (notably macOS first-launch validation). Keep the probe
// bounded without rejecting a compatible Engine just because its launch is cold.
const RUNTIME_INFO_TIMEOUT: Duration = Duration::from_secs(15);
/// The largest `--runtime-info` response read from a probed binary.
const RUNTIME_INFO_MAX_BYTES: usize = 8192;

/// Probe the executable before creating local data or starting services. Package versions
/// do not distinguish the unreleased v3 architecture from released v2 binaries.
async fn ensure_compatible_binary(path: &Path) -> Result<()> {
	let incompatible = |reason: &str| {
		EngineProcessError::IncompatibleBinary {
			path: path.display().to_string(),
			reason: reason.to_owned(),
			required: ENGINE_ARCHITECTURE.to_owned(),
		}
		.build()
	};
	let mut child = Command::new(path)
		.arg("--runtime-info")
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.kill_on_drop(true)
		.spawn()
		.with_context(|| format!("probe engine binary `{}`", path.display()))?;
	let stdout = child.stdout.take().context("probe stdout was not piped")?;
	let probe = async {
		let read = async {
			let mut bytes = Vec::new();
			let limit = RUNTIME_INFO_MAX_BYTES as u64 + 1;
			stdout.take(limit).read_to_end(&mut bytes).await?;
			if bytes.len() > RUNTIME_INFO_MAX_BYTES {
				return Err(std::io::Error::other(
					"runtime identity response exceeds its size limit",
				));
			}
			Ok(bytes)
		};
		tokio::try_join!(child.wait(), read)
	};
	let (status, bytes) = match tokio::time::timeout(RUNTIME_INFO_TIMEOUT, probe).await {
		Ok(Ok(output)) => output,
		failure => {
			// Kill and reap before reporting failure; kill_on_drop also covers caller cancellation.
			child
				.kill()
				.await
				.context("terminate failed runtime identity probe")?;
			return Err(incompatible(if failure.is_err() {
				"runtime identity probe timed out"
			} else {
				"runtime identity probe output could not be read"
			}));
		}
	};
	if !status.success() {
		return Err(incompatible(
			"runtime identity probe is unsupported or failed",
		));
	}
	let info: EngineHealthResponse = serde_json::from_slice(&bytes)
		.map_err(|_| incompatible("runtime identity response is invalid"))?;
	if info.runtime.as_deref() != Some(ENGINE_RUNTIME)
		|| info.architecture.as_deref() != Some(ENGINE_ARCHITECTURE)
	{
		return Err(incompatible(
			"runtime identity does not advertise the required architecture",
		));
	}
	Ok(())
}

/// Path to the local engine's Depot data under the shared storage root. It holds the log
/// shard and the object store, and is kept apart from the `db` directory older engines used.
pub fn engine_data_path() -> Result<PathBuf> {
	Ok(storage_root()?.join("var").join("engine").join("depot"))
}

/// The local engine's log shard directory, which must exist before the engine starts. The
/// engine creates its object store directory itself.
pub fn engine_log_directory() -> Result<PathBuf> {
	Ok(engine_data_path()?.join("log"))
}

fn admin_token() -> String {
	env::var("RIVET__AUTH__ADMIN_TOKEN").unwrap_or_else(|_| "default".to_owned())
}

fn engine_stamp_path() -> Result<PathBuf> {
	Ok(storage_root()?
		.join("var")
		.join("engine")
		.join("runtime.json"))
}

/// The address the engine actually binds: the explicit `bind_host` override, or
/// the endpoint host when unset. Must match `engine_env`'s guard host.
fn resolve_bind_host(config: &EngineResolverConfig) -> Result<String> {
	if let Some(bind_host) = &config.bind_host {
		return Ok(bind_host.clone());
	}
	let endpoint = &config.endpoint;
	let endpoint_url =
		Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
	Ok(endpoint_url
		.host_str()
		.ok_or_else(|| invalid_endpoint(endpoint, "missing host"))?
		.to_owned())
}

fn write_engine_stamp(stamp: &EngineRuntimeStamp) -> Result<()> {
	let path = engine_stamp_path()?;
	if let Some(dir) = path.parent() {
		ensure_dir(dir)?;
	}
	let contents = serde_json::to_vec_pretty(stamp).context("serialize engine stamp")?;
	std::fs::write(&path, contents)
		.with_context(|| format!("write engine stamp `{}`", path.display()))?;
	Ok(())
}

/// Reads the runtime stamp of the currently running engine, if its recorded
/// process is still alive. A stale stamp (process gone) reads as `None` so
/// callers treat the binding as unknown rather than trusting old data. On
/// platforms without a liveness check (see `pid_is_alive`) the stamp is always
/// treated as unknown for the same reason.
fn read_engine_stamp() -> Option<EngineRuntimeStamp> {
	let path = engine_stamp_path().ok()?;
	read_engine_stamp_from(&path)
}

fn read_engine_stamp_from(path: &Path) -> Option<EngineRuntimeStamp> {
	let contents = std::fs::read(path).ok()?;
	let stamp: EngineRuntimeStamp = serde_json::from_slice(&contents).ok()?;
	if !pid_is_alive(stamp.pid) {
		let _ = std::fs::remove_file(path);
		return None;
	}
	Some(stamp)
}

#[cfg(unix)]
fn pid_is_alive(pid: u32) -> bool {
	// `kill(pid, 0)` sends no signal but performs the existence and permission
	// checks: success or EPERM means the process exists, ESRCH means it is gone.
	let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
	result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn pid_is_alive(_pid: u32) -> bool {
	// No portable liveness check here, so fail safe: report the process as gone
	// so a stale stamp is never trusted. The binding check then falls back to
	// its "unknown, warn" branch instead of silently trusting old data.
	false
}

/// Computes the environment variables that configure a rivet-engine process
/// for the given endpoint. The engine requires the log directory to exist; see
/// [`engine_log_directory`].
///
/// Shared by the spawn path and by callers that exec the engine binary
/// directly (for example the CLI `engine` proxy) so both operate on the same
/// data, guard, api-peer, and metrics ports. These callers run one local
/// development engine: it is the only Depot storage worker, keeps its cold
/// data in a local directory, provisions its own tablet on first start, and uses
/// shorter recovery and shutdown thresholds than the production defaults.
pub fn engine_env(config: &EngineResolverConfig) -> Result<Vec<(String, String)>> {
	let ports = EnginePorts::resolve(config)?;
	let log_directory = engine_log_directory()?;
	let object_store = engine_data_path()?.join("objects");

	let env = [
		("RIVET__AUTH__ADMIN_TOKEN", admin_token()),
		("RIVET__GUARD__HOST", ports.guard_host.clone()),
		("RIVET__GUARD__PORT", ports.guard.to_string()),
		// The peer listener only serves this engine's own internal requests.
		("RIVET__API_PEER__HOST", LOOPBACK.to_owned()),
		("RIVET__API_PEER__PORT", ports.api_peer.to_string()),
		("RIVET__METRICS__HOST", ports.guard_host),
		("RIVET__METRICS__PORT", ports.metrics.to_string()),
		(
			"RIVET__STORAGE__LOG_DIRECTORY",
			log_directory.to_string_lossy().into_owned(),
		),
		(
			"RIVET__OBJECT_STORAGE__PRIMARY__FILE_SYSTEM__PATH",
			object_store.to_string_lossy().into_owned(),
		),
		// One storage worker holds every tablet, so there is nothing to place or split.
		("RIVET__STORAGE__BALANCING__ENABLED", "false".to_owned()),
		("RIVET__STORAGE__AUTO_PROVISION_PARTITIONS", "1".to_owned()),
		// Bound disk use instead of sizing from the host's free space. The cold store is
		// already on local disk, so a disk cache in front of it only duplicates data.
		(
			"RIVET__STORAGE__LOG_SHARD__MAX_DISK_BYTES",
			LOG_MAX_DISK_BYTES.to_string(),
		),
		(
			"RIVET__STORAGE__COLD_STORE__DISK_CACHE_BYTES",
			"0".to_owned(),
		),
		// Storage shutdown must outlast the cold-write age, and the forced shutdown must
		// outlast both stop budgets; these keep a local engine's stop within seconds.
		("RIVET__STORAGE__WRITE_TO_COLD_AFTER_MS", "5000".to_owned()),
		("RIVET__STORAGE__STOP_TIMEOUT_MS", "6000".to_owned()),
		("RIVET__STORAGE__STOP_TEARDOWN_MS", "2000".to_owned()),
		("RIVET__PEGBOARD__NODE_LOST_THRESHOLD", "7000".to_owned()),
		("RIVET__RUNTIME__WORKER_SHUTDOWN_DURATION", "1".to_owned()),
		("RIVET__RUNTIME__GUARD_SHUTDOWN_DURATION", "1".to_owned()),
		("RIVET__RUNTIME__FORCE_SHUTDOWN_DURATION", "10".to_owned()),
	];
	Ok(env
		.into_iter()
		.map(|(key, value)| (key.to_owned(), value))
		.collect())
}

const LOOPBACK: &str = "127.0.0.1";

/// Log shard disk cap of the local engine.
const LOG_MAX_DISK_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Listener addresses derived from the endpoint: guard on its port, the peer listener one
/// above it and metrics ten above it.
struct EnginePorts {
	guard_host: String,
	guard: u16,
	api_peer: u16,
	metrics: u16,
}

impl EnginePorts {
	fn resolve(config: &EngineResolverConfig) -> Result<Self> {
		let endpoint = &config.endpoint;
		let endpoint_url =
			Url::parse(endpoint).with_context(|| format!("parse engine endpoint `{endpoint}`"))?;
		let guard_port = endpoint_url
			.port_or_known_default()
			.ok_or_else(|| invalid_endpoint(endpoint, "missing port"))?;
		let guard = config.bind_port.unwrap_or(guard_port);
		Ok(Self {
			guard_host: resolve_bind_host(config)?,
			guard,
			api_peer: guard
				.checked_add(1)
				.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?,
			metrics: guard
				.checked_add(10)
				.ok_or_else(|| invalid_endpoint(endpoint, "port is too large"))?,
		})
	}
}

/// Writes the topology, which the engine cannot take from environment variables. The public
/// URL is the explicit override or the endpoint; the peer URL is the loopback peer listener.
fn write_engine_config(config: &EngineResolverConfig) -> Result<PathBuf> {
	let public_url = config.public_url.as_deref().unwrap_or(&config.endpoint);
	let public_url = Url::parse(public_url)
		.with_context(|| format!("parse engine public URL `{public_url}`"))?;
	let ports = EnginePorts::resolve(config)?;
	let peer_url = format!("http://{LOOPBACK}:{}", ports.api_peer);
	let dir = storage_root()?.join("var").join("engine");
	ensure_dir(&dir)?;
	let path = dir.join("config.json");
	let config = serde_json::json!({
		"topology": {
			"datacenter_label": 1,
			"datacenters": {
				"default": {
					"datacenter_label": 1,
					"is_leader": true,
					"public_url": public_url.as_str(),
					"peer_url": peer_url,
				}
			}
		}
	});
	std::fs::write(&path, serde_json::to_string_pretty(&config)?)
		.with_context(|| format!("write engine config `{}`", path.display()))?;
	Ok(path)
}

pub async fn resolve_engine_binary(config: &EngineResolverConfig) -> Result<ResolvedEngine> {
	if let Some(path) = config.explicit_binary_path.as_ref() {
		return verify_binary_path(path);
	}

	if let Some(path) = std::env::var_os("RIVET_ENGINE_BINARY_PATH").map(PathBuf::from) {
		return verify_binary_path(&path);
	}

	if probe_existing_engine(&config.endpoint).await?.is_some() {
		return Ok(ResolvedEngine::Existing);
	}

	let local_roots = local_engine_search_roots();
	let cached = cached_engine_path(&config.version)?;
	resolve_engine_binary_after_probe(config, false, &local_roots, cached).await
}

/// Resolves the engine binary path without probing for an already-running
/// engine.
///
/// Used by callers that exec the binary directly (for example the CLI `engine`
/// proxy), where a running engine on the endpoint should not short-circuit
/// resolution to `Existing` and leave the caller without a path to run.
pub async fn resolve_engine_binary_path(config: &EngineResolverConfig) -> Result<PathBuf> {
	if let Some(path) = config.explicit_binary_path.as_ref() {
		verify_binary_path(path)?;
		return Ok(path.clone());
	}

	if let Some(path) = std::env::var_os("RIVET_ENGINE_BINARY_PATH").map(PathBuf::from) {
		verify_binary_path(&path)?;
		return Ok(path);
	}

	let local_roots = local_engine_search_roots();
	let cached = cached_engine_path(&config.version)?;
	match resolve_engine_binary_after_probe(config, false, &local_roots, cached).await? {
		ResolvedEngine::Binary(path) => Ok(path),
		ResolvedEngine::Existing => {
			unreachable!("no-probe resolution never returns Existing")
		}
	}
}

async fn resolve_engine_binary_after_probe(
	config: &EngineResolverConfig,
	existing_engine: bool,
	local_roots: &[PathBuf],
	cached: PathBuf,
) -> Result<ResolvedEngine> {
	if existing_engine {
		return Ok(ResolvedEngine::Existing);
	}

	if let Some(path) = find_local_engine_binary_in_roots(local_roots) {
		return Ok(ResolvedEngine::Binary(path));
	}

	if cached.exists() {
		return Ok(ResolvedEngine::Binary(cached));
	}

	if !config.auto_download {
		return Err(EngineProcessError::BinaryUnavailable {
			version: config.version.clone(),
		}
		.build());
	}

	download_engine_binary(config, &cached).await?;
	Ok(ResolvedEngine::Binary(cached))
}

fn verify_binary_path(path: &Path) -> Result<ResolvedEngine> {
	if !path.exists() {
		return Err(EngineProcessError::BinaryNotFound {
			path: path.display().to_string(),
		}
		.build());
	}
	Ok(ResolvedEngine::Binary(path.to_path_buf()))
}

fn local_engine_search_roots() -> Vec<PathBuf> {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.map(Path::to_path_buf)
		.collect()
}

fn find_local_engine_binary_in_roots(roots: &[PathBuf]) -> Option<PathBuf> {
	for root in roots {
		for profile in ["debug", "release"] {
			let candidate = root
				.join("target")
				.join(profile)
				.join(exe_name("rivet-engine"));
			if candidate.exists() {
				return Some(candidate);
			}
		}
	}
	None
}

/// Cached release binaries live under the architecture, so a v2 engine cached by an older
/// RivetKit under the same version number is never picked up.
fn cached_engine_path(version: &str) -> Result<PathBuf> {
	Ok(storage_root()?
		.join("engine")
		.join(ENGINE_ARCHITECTURE)
		.join(version)
		.join(engine_artifact_name()))
}

async fn download_engine_binary(config: &EngineResolverConfig, destination: &Path) -> Result<()> {
	let artifact = engine_artifact_name();
	let base = config.releases_endpoint.trim_end_matches('/');
	let artifact_url = format!("{base}/rivet/{}/engine/{artifact}", config.version);
	let manifest_url = format!("{base}/rivet/{}/engine/SHA256SUMS", config.version);
	let client = Client::builder()
		.timeout(DOWNLOAD_TIMEOUT)
		.build()
		.context("build reqwest client for engine download")?;

	let manifest = fetch_text(&client, &manifest_url).await?;
	let expected = checksum_for_artifact(&manifest, &artifact).ok_or_else(|| {
		EngineProcessError::DownloadFailed {
			url: manifest_url.clone(),
			reason: format!("manifest does not contain `{artifact}`"),
		}
		.build()
	})?;

	let bytes = fetch_bytes(&client, &artifact_url).await?;
	let received = sha256_hex(&bytes);
	if !received.eq_ignore_ascii_case(&expected) {
		return Err(EngineProcessError::ChecksumMismatch {
			artifact,
			expected,
			received,
		}
		.build());
	}

	let parent = destination
		.parent()
		.context("engine cache destination has no parent")?;
	ensure_dir(parent)?;
	std::fs::write(destination, bytes)
		.with_context(|| format!("write engine binary `{}`", destination.display()))?;
	make_executable(destination)?;
	Ok(())
}

async fn fetch_text(client: &Client, url: &str) -> Result<String> {
	let response = client.get(url).send().await.map_err(|error| {
		EngineProcessError::DownloadFailed {
			url: url.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})?;
	if !response.status().is_success() {
		let status = response.status();
		return Err(EngineProcessError::DownloadFailed {
			url: url.to_owned(),
			reason: format!("unexpected status {status}"),
		}
		.build());
	}
	response.text().await.map_err(|error| {
		EngineProcessError::DownloadFailed {
			url: url.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})
}

async fn fetch_bytes(client: &Client, url: &str) -> Result<Vec<u8>> {
	let response = client.get(url).send().await.map_err(|error| {
		EngineProcessError::DownloadFailed {
			url: url.to_owned(),
			reason: error.to_string(),
		}
		.build()
	})?;
	if !response.status().is_success() {
		let status = response.status();
		return Err(EngineProcessError::DownloadFailed {
			url: url.to_owned(),
			reason: format!("unexpected status {status}"),
		}
		.build());
	}
	response
		.bytes()
		.await
		.map(|bytes| bytes.to_vec())
		.map_err(|error| {
			EngineProcessError::DownloadFailed {
				url: url.to_owned(),
				reason: error.to_string(),
			}
			.build()
		})
}

fn checksum_for_artifact(manifest: &str, artifact: &str) -> Option<String> {
	manifest.lines().find_map(|line| {
		let mut parts = line.split_whitespace();
		let checksum = parts.next()?;
		let name = parts.next()?.trim_start_matches('*');
		(checksum.len() == 64 && name == artifact).then(|| checksum.to_owned())
	})
}

fn sha256_hex(bytes: &[u8]) -> String {
	let digest = Sha256::digest(bytes);
	let mut out = String::with_capacity(digest.len() * 2);
	for byte in digest {
		use std::fmt::Write;
		let _ = write!(&mut out, "{byte:02x}");
	}
	out
}

fn engine_artifact_name() -> String {
	let arch = match std::env::consts::ARCH {
		"x86_64" => "x86_64",
		"aarch64" => "aarch64",
		other => other,
	};
	let target = match std::env::consts::OS {
		"linux" => format!("{arch}-unknown-linux-musl"),
		"macos" => format!("{arch}-apple-darwin"),
		"windows" => format!("{arch}-pc-windows-gnu.exe"),
		other => format!("{arch}-{other}"),
	};
	format!("rivet-engine-{target}")
}

fn exe_name(base: &str) -> String {
	if cfg!(windows) {
		format!("{base}.exe")
	} else {
		base.to_owned()
	}
}

fn make_executable(path: &Path) -> Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut permissions = std::fs::metadata(path)
			.with_context(|| format!("read metadata for `{}`", path.display()))?
			.permissions();
		permissions.set_mode(0o755);
		std::fs::set_permissions(path, permissions)
			.with_context(|| format!("mark `{}` executable", path.display()))?;
	}
	#[cfg(not(unix))]
	{
		let _ = path;
	}
	Ok(())
}

impl Drop for EngineProcessManager {
	fn drop(&mut self) {
		if let Some(stop) = self.stop.take() {
			let _ = stop.send(());
		}
	}
}

/// Spawns a background task that owns the `Child` and awaits `wait()` so we
/// log a clear message if the engine dies while rivetkit is still up. Taking
/// the `Child` into the task also reaps it via `waitpid` on exit, so a
/// crashed engine never lingers as a zombie in our process table.
fn spawn_engine_watcher(mut child: Child, pid: u32) -> (JoinHandle<()>, oneshot::Sender<()>) {
	let (stop, stop_rx) = oneshot::channel();
	let watcher = tokio::spawn(async move {
		let exit = tokio::select! {
			exit = child.wait() => Some(exit),
			_ = stop_rx => None,
		};
		if exit.is_none() {
			if matches!(child.try_wait(), Ok(None)) {
				#[cfg(unix)]
				unsafe {
					libc::kill(pid as libc::pid_t, libc::SIGTERM);
				}
				#[cfg(not(unix))]
				let _ = child.start_kill();
			}
			match tokio::time::timeout(Duration::from_secs(15), child.wait()).await {
				Ok(Ok(_)) => {}
				result => {
					tracing::warn!(
						pid,
						?result,
						"engine did not stop after SIGTERM; killing it"
					);
					let _ = child.start_kill();
					let _ = child.wait().await;
				}
			}
			return;
		}
		match exit.expect("checked above") {
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
	});
	(watcher, stop)
}

/// Probes the configured endpoint for an already-running, healthy engine.
///
/// Returns `Ok(Some(health))` if the endpoint is serving a `runtime: "engine"`
/// health response that we can reattach to. Returns `Ok(None)` if the port is
/// free. Returns `Err(...)` if the port is occupied by a non-engine process
/// (for example a stale rivetkit) which would conflict with a fresh spawn.
async fn probe_existing_engine(endpoint: &str) -> Result<Option<EngineHealthResponse>> {
	let health_url = engine_health_url(endpoint);
	let client = Client::builder()
		.build()
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
		Some(ENGINE_RUNTIME) => {
			ensure_compatible_engine(endpoint, &health)?;
			Ok(Some(health))
		}
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

/// Refuses any engine that does not report the required architecture, so a v2 engine
/// listening on the endpoint is never reused or left serving.
fn ensure_compatible_engine(endpoint: &str, health: &EngineHealthResponse) -> Result<()> {
	if health.architecture.as_deref() == Some(ENGINE_ARCHITECTURE) {
		return Ok(());
	}
	Err(EngineProcessError::IncompatibleEngine {
		endpoint: endpoint.to_owned(),
		version: health
			.version
			.clone()
			.unwrap_or_else(|| "unknown".to_owned()),
		architecture: health
			.architecture
			.clone()
			.unwrap_or_else(|| "v2".to_owned()),
		required: ENGINE_ARCHITECTURE.to_owned(),
	}
	.build())
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
	// A first start creates the log shard before the HTTP listener binds.
	const HEALTH_MAX_WAIT: Duration = Duration::from_secs(60);
	const HEALTH_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
	const HEALTH_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
	const HEALTH_MAX_BACKOFF: Duration = Duration::from_secs(1);

	let client = Client::builder()
		.build()
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

/// Waits until the engine routes requests to Depot. Guard answers health before the
/// first tablet is provisioned and opened, and actor requests fail until then, so this
/// reads an actor listing, which routes through Depot, until it succeeds.
async fn wait_for_engine_ready(endpoint: &str) -> Result<()> {
	const READY_MAX_WAIT: Duration = Duration::from_secs(60);
	const READY_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
	const READY_BACKOFF: Duration = Duration::from_millis(250);

	let url = format!(
		"{}/actors?namespace=default&name=rivetkit-readiness-probe&key=probe",
		endpoint.trim_end_matches('/')
	);
	let token = admin_token();
	let client = Client::builder()
		.build()
		.context("build reqwest client for engine readiness check")?;
	let deadline = Instant::now() + READY_MAX_WAIT;
	let mut attempt = 0u32;

	loop {
		attempt += 1;

		let last_error = match client
			.get(&url)
			.bearer_auth(&token)
			.timeout(READY_REQUEST_TIMEOUT)
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => return Ok(()),
			Ok(response) => {
				let status = response.status();
				let body = response.text().await.unwrap_or_default();
				format!("unexpected status {status}: {body}")
			}
			Err(error) => error.to_string(),
		};
		tracing::debug!(attempt, reason = %last_error, "waiting for engine readiness");

		if Instant::now() >= deadline {
			return Err(EngineProcessError::NotReady {
				attempts: attempt,
				reason: last_error,
			}
			.build());
		}

		tokio::time::sleep(READY_BACKOFF).await;
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


