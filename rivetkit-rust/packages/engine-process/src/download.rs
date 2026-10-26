//! On-demand download of release binaries (the Rivet Engine and Services)
//! from releases.rivet.dev for local development.
//!
//! Binaries are cached per user, keyed by product, version, and platform. A
//! download is verified against the release's `SHA256SUMS` manifest before it
//! is atomically renamed into the cache, and a per-binary file lock keeps
//! parallel processes from corrupting or duplicating the download.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::{Client, StatusCode, Url};
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Base URL that hosts Rivet release artifacts.
pub const RELEASES_ENDPOINT: &str = "https://releases.rivet.dev";
/// Set to `1` to forbid all binary downloads.
pub const DISABLE_DOWNLOAD_ENV: &str = "RIVET_DISABLE_BINARY_DOWNLOAD";
/// Turns binary downloads on (`1`) or off (`0`).
pub const AUTO_DOWNLOAD_ENV: &str = "RIVETKIT_ENGINE_AUTO_DOWNLOAD";
/// Overrides the per-user cache directory that downloaded binaries live in.
pub const CACHE_DIR_ENV: &str = "RIVET_BINARY_CACHE_DIR";
const CHECKSUM_MANIFEST: &str = "SHA256SUMS";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Maximum time to wait for the next chunk of the binary before giving up.
const CHUNK_TIMEOUT: Duration = Duration::from_secs(60);
/// How often a running download logs its progress.
const PROGRESS_INTERVAL: Duration = Duration::from_secs(10);
/// Retries after the first attempt for transient download failures.
const DOWNLOAD_RETRIES: u32 = 3;
/// Delay before the first retry. It doubles for each later retry.
const RETRY_BASE_DELAY: Duration = if cfg!(test) {
	Duration::from_millis(10)
} else {
	Duration::from_secs(1)
};
/// Longest `Retry-After` delay honored for a rate-limited download.
const MAX_RETRY_AFTER: Duration = if cfg!(test) {
	Duration::from_millis(50)
} else {
	Duration::from_secs(30)
};
/// Maximum time to wait for another process that is downloading the same binary.
const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// How often to retry a contended download lock. File locks have no async
/// wakeup, so the waiter retries `try_lock` on this interval.
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(500);

/// A release binary that can be downloaded on demand.
#[derive(Clone, Debug)]
pub struct ReleaseBinary {
	/// Product name shown in logs, for example `Rivet Engine`.
	pub display_name: &'static str,
	/// Cache subdirectory, for example `engine`.
	pub cache_name: &'static str,
	/// Exact release version to download. Never `latest`.
	pub version: String,
	/// Platform label shown in logs and used as a cache key.
	pub platform: String,
	/// Directory URL that holds the artifact and its `SHA256SUMS` manifest.
	pub base_url: String,
	/// File name of the artifact in `base_url`.
	pub artifact: String,
	/// File name of the binary in the cache.
	pub file_name: String,
	/// Environment variable that points at a pre-downloaded binary instead.
	pub override_env: &'static str,
	/// Extra workaround offered when the download fails.
	pub alternative: &'static str,
}

impl ReleaseBinary {
	pub fn artifact_url(&self) -> String {
		format!("{}/{}", self.base_url.trim_end_matches('/'), self.artifact)
	}

	pub fn manifest_url(&self) -> String {
		format!(
			"{}/{CHECKSUM_MANIFEST}",
			self.base_url.trim_end_matches('/')
		)
	}

	/// Final location of the verified binary under `cache_root`.
	pub fn cache_path(&self, cache_root: &Path) -> PathBuf {
		cache_root
			.join(self.cache_name)
			.join(&self.version)
			.join(&self.platform)
			.join(&self.file_name)
	}

	/// Whether `cache_root` holds a complete, verified copy of this binary.
	///
	/// A cached binary counts only when its checksum record exists and its size
	/// matches the verified download, so a truncated or replaced file is
	/// downloaded again instead of being run.
	pub fn is_cached(&self, cache_root: &Path) -> bool {
		cached_binary_is_valid(&self.cache_path(cache_root))
	}

	/// Ways to avoid downloading this binary, shown once for slow downloads.
	fn skip_hint(&self) -> String {
		format!(
			"set {} to a pre-downloaded binary, set {CACHE_DIR_ENV} to a cache directory that persists across runs, or {}",
			self.override_env, self.alternative
		)
	}

	fn workaround(&self) -> String {
		format!(
			"Download {} manually, verify it against {}, and set {} to its path, or {}",
			self.artifact_url(),
			self.manifest_url(),
			self.override_env,
			self.alternative
		)
	}
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("binary_download")]
pub enum BinaryDownloadError {
	#[error(
		"disabled",
		"Binary downloads are disabled.",
		"{binary} {version} is not cached and downloads are disabled by {env}=1. {workaround}"
	)]
	Disabled {
		binary: String,
		version: String,
		env: String,
		workaround: String,
	},

	#[error(
		"unsupported_platform",
		"No release binary exists for this platform.",
		"{binary} is not published for {os}-{arch}. Set {override_env} to a binary built for this platform, or {alternative}"
	)]
	UnsupportedPlatform {
		binary: String,
		os: String,
		arch: String,
		override_env: String,
		alternative: String,
	},

	#[error(
		"request_failed",
		"Binary download request failed.",
		"Could not download {binary} {version} from {url}: {reason}. Check your network connection or proxy settings. {workaround}"
	)]
	RequestFailed {
		binary: String,
		version: String,
		url: String,
		reason: String,
		workaround: String,
	},

	#[error(
		"http_status",
		"Binary download returned an HTTP error.",
		"Could not download {binary} {version} from {url}: HTTP {status}. {workaround}"
	)]
	HttpStatus {
		binary: String,
		version: String,
		url: String,
		status: u16,
		workaround: String,
	},

	#[error(
		"checksum_missing",
		"Binary checksum is missing from the release manifest.",
		"The release manifest {url} has no checksum for {artifact}, so {binary} {version} cannot be verified and will not run. {workaround}"
	)]
	ChecksumMissing {
		binary: String,
		version: String,
		url: String,
		artifact: String,
		workaround: String,
	},

	#[error(
		"checksum_mismatch",
		"Binary checksum mismatch.",
		"Downloaded {binary} {version} from {url} does not match its published SHA-256 checksum (expected {expected}, received {received}). The file was discarded. Retry, or {workaround}"
	)]
	ChecksumMismatch {
		binary: String,
		version: String,
		url: String,
		expected: String,
		received: String,
		workaround: String,
	},

	#[error(
		"incomplete",
		"Binary download was incomplete.",
		"Download of {binary} {version} from {url} ended after {received_bytes} of {expected_bytes} bytes. The partial file was discarded. Retry, or {workaround}"
	)]
	Incomplete {
		binary: String,
		version: String,
		url: String,
		received_bytes: u64,
		expected_bytes: u64,
		workaround: String,
	},

	#[error(
		"invalid_version",
		"Binary version is invalid.",
		"{binary} version `{version}` is not a valid release version. A version may contain only ASCII letters, digits, `.`, `-`, `+`, and `_`, and must start with a letter or digit."
	)]
	InvalidVersion { binary: String, version: String },

	#[error(
		"unpinned_version",
		"Binary version is not pinned.",
		"{binary} version `{version}` is not a pinned release version. Set an exact version such as `2.3.30`. Aliases such as `latest` are not supported because a cached alias would never update."
	)]
	UnpinnedVersion { binary: String, version: String },

	#[error(
		"lock_timeout",
		"Timed out waiting for another binary download.",
		"Timed out after {timeout_secs}s waiting for another process to finish downloading {binary} {version}. That process holds the lock `{lock_path}`. Stop it and retry, or {workaround}"
	)]
	LockTimeout {
		binary: String,
		version: String,
		lock_path: String,
		timeout_secs: u64,
		workaround: String,
	},

	#[error(
		"io",
		"Binary cache could not be written.",
		"Could not {operation} `{path}` while installing {binary} {version}: {reason}. Check that the directory is writable and has free space, set {cache_env} to a writable directory, or {workaround}"
	)]
	Io {
		binary: String,
		version: String,
		operation: String,
		path: String,
		reason: String,
		cache_env: String,
		workaround: String,
	},
}

/// Whether `RIVET_DISABLE_BINARY_DOWNLOAD` forbids downloads.
pub fn download_disabled_by_env() -> bool {
	std::env::var(DISABLE_DOWNLOAD_ENV)
		.map(|value| is_truthy(&value))
		.unwrap_or(false)
}

/// Whether a missing binary may be downloaded, from the environment.
///
/// `RIVET_DISABLE_BINARY_DOWNLOAD=1` always wins. Otherwise
/// `RIVETKIT_ENGINE_AUTO_DOWNLOAD` can turn downloads on or off, and `default`
/// applies when neither is set.
pub fn auto_download_from_env(default: bool) -> bool {
	resolve_auto_download(None, default, |key| std::env::var(key).ok())
}

/// Whether a missing binary may be downloaded.
///
/// `RIVET_DISABLE_BINARY_DOWNLOAD=1` always wins. Otherwise the explicit config
/// value applies, then `RIVETKIT_ENGINE_AUTO_DOWNLOAD`, then `default`.
pub fn resolve_auto_download(
	explicit: Option<bool>,
	default: bool,
	env: impl Fn(&str) -> Option<String>,
) -> bool {
	if env(DISABLE_DOWNLOAD_ENV).is_some_and(|value| is_truthy(&value)) {
		return false;
	}
	if let Some(explicit) = explicit {
		return explicit;
	}
	match env(AUTO_DOWNLOAD_ENV) {
		Some(value) if !value.trim().is_empty() => is_truthy(&value),
		Some(_) | None => default,
	}
}

/// Whether an environment flag is on: `1`, `true`, `yes`, or `on`.
pub fn is_truthy(value: &str) -> bool {
	matches!(
		value.trim().to_ascii_lowercase().as_str(),
		"1" | "true" | "yes" | "on"
	)
}

/// Rejects release versions that could escape the cache directory or the
/// release URL, such as `..` or values containing path separators.
pub fn validate_version(display_name: &str, version: &str) -> Result<()> {
	let starts_alphanumeric = version
		.chars()
		.next()
		.is_some_and(|first| first.is_ascii_alphanumeric());
	let allowed = version
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
	if starts_alphanumeric && allowed {
		return Ok(());
	}
	Err(BinaryDownloadError::InvalidVersion {
		binary: display_name.to_owned(),
		version: version.to_owned(),
	}
	.build())
}

/// Rejects versions that are not exact releases, such as `latest` or `main`.
/// Exact versions start with a digit, for example `2.3.30` or `2.3.30-rc.1`.
pub fn validate_pinned_version(display_name: &str, version: &str) -> Result<()> {
	validate_version(display_name, version)?;
	if version.starts_with(|c: char| c.is_ascii_digit()) {
		return Ok(());
	}
	Err(BinaryDownloadError::UnpinnedVersion {
		binary: display_name.to_owned(),
		version: version.to_owned(),
	}
	.build())
}

/// Per-user cache root for downloaded binaries.
///
/// Honors `RIVET_BINARY_CACHE_DIR`, then the platform cache directory
/// (`$XDG_CACHE_HOME` or `~/.cache` on Linux, `~/Library/Caches` on macOS,
/// `%LOCALAPPDATA%` on Windows), then `~/.cache`.
pub fn binary_cache_root() -> Result<PathBuf> {
	if let Some(path) = std::env::var_os(CACHE_DIR_ENV).filter(|value| !value.is_empty()) {
		return Ok(PathBuf::from(path));
	}
	if let Some(path) = dirs::cache_dir() {
		return Ok(path.join("rivet"));
	}
	let home = dirs::home_dir().ok_or_else(|| {
		anyhow::anyhow!(
			"could not locate a cache directory for downloaded binaries; set {CACHE_DIR_ENV}"
		)
	})?;
	Ok(home.join(".cache").join("rivet"))
}

/// Returns the cached binary, downloading and verifying it first if needed.
///
/// `reason` explains in logs why the binary is needed. When `allow_download`
/// is false, a missing binary fails with an error that explains what to set
/// instead. Every failure is logged once here with its resolution.
pub async fn ensure_release_binary(
	binary: &ReleaseBinary,
	cache_root: &Path,
	allow_download: bool,
	reason: &'static str,
) -> Result<PathBuf> {
	let result = install_release_binary(binary, cache_root, allow_download, reason).await;
	if let Err(error) = &result {
		tracing::error!(
			binary = binary.display_name,
			version = %binary.version,
			%error,
			"could not install binary"
		);
	}
	result
}

async fn install_release_binary(
	binary: &ReleaseBinary,
	cache_root: &Path,
	allow_download: bool,
	reason: &'static str,
) -> Result<PathBuf> {
	validate_version(binary.display_name, &binary.version)?;
	let destination = binary.cache_path(cache_root);
	if cached_binary_is_valid(&destination) {
		tracing::debug!(
			binary = binary.display_name,
			version = %binary.version,
			path = %destination.display(),
			"using cached binary"
		);
		return Ok(destination);
	}

	if !allow_download {
		return Err(BinaryDownloadError::Disabled {
			binary: binary.display_name.to_owned(),
			version: binary.version.clone(),
			env: DISABLE_DOWNLOAD_ENV.to_owned(),
			workaround: binary.workaround(),
		}
		.build());
	}

	let directory = destination
		.parent()
		.ok_or_else(|| io_error(binary, "resolve the parent of", &destination, "no parent"))?
		.to_path_buf();
	std::fs::create_dir_all(&directory)
		.map_err(|error| io_error(binary, "create directory", &directory, error))?;

	let lock_path = directory.join(format!("{}.lock", binary.file_name));
	let lock = acquire_lock(binary, &lock_path, LOCK_WAIT_TIMEOUT).await?;

	// Another process may have finished the download while this one waited.
	if cached_binary_is_valid(&destination) {
		tracing::info!(
			binary = binary.display_name,
			version = %binary.version,
			path = %destination.display(),
			"binary was downloaded by another process"
		);
		return Ok(destination);
	}

	if destination.exists() {
		tracing::warn!(
			binary = binary.display_name,
			version = %binary.version,
			path = %destination.display(),
			"cached binary is incomplete or unverified, likely from an interrupted install or a modified cache; downloading it again"
		);
	}

	// Without the lock another process may own a temp file in this directory.
	if lock.is_some() {
		remove_stale_downloads(&directory, &binary.file_name);
	}
	download_verified(binary, &destination, reason).await?;
	drop(lock);
	Ok(destination)
}

/// Path of the checksum record written next to a verified binary.
pub(crate) fn checksum_record_path(destination: &Path) -> PathBuf {
	let mut name = destination
		.file_name()
		.map(|name| name.to_os_string())
		.unwrap_or_default();
	name.push(".sha256");
	destination.with_file_name(name)
}

/// Whether `destination` exists and matches the size in its checksum record.
fn cached_binary_is_valid(destination: &Path) -> bool {
	let Ok(metadata) = std::fs::metadata(destination) else {
		return false;
	};
	if !metadata.is_file() {
		return false;
	}
	let Ok(record) = std::fs::read_to_string(checksum_record_path(destination)) else {
		return false;
	};
	let mut parts = record.split_whitespace();
	let checksum_ok = parts.next().is_some_and(|checksum| checksum.len() == 64);
	let size = parts.next().and_then(|size| size.parse::<u64>().ok());
	checksum_ok && size == Some(metadata.len())
}

/// Removes a temporary download when dropped, including when the download
/// future is cancelled, unless the file was installed.
struct TempDownload {
	path: PathBuf,
	installed: bool,
}

impl Drop for TempDownload {
	fn drop(&mut self) {
		if !self.installed && std::fs::remove_file(&self.path).is_ok() {
			tracing::info!(
				path = %self.path.display(),
				"discarded incomplete download"
			);
		}
	}
}

/// Unique temp file next to `destination`. The process id and a timestamp keep
/// concurrent downloads apart when locking is unavailable.
fn temp_download_path(destination: &Path, file_name: &str) -> PathBuf {
	let nanos = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|elapsed| elapsed.as_nanos())
		.unwrap_or_default();
	destination.with_file_name(format!(
		".{file_name}.{}-{nanos}.download",
		std::process::id()
	))
}

/// Removes temporary files left by downloads that were interrupted, for
/// example when the process was killed. Only called while holding the
/// download lock, so no live download owns them.
fn remove_stale_downloads(directory: &Path, file_name: &str) {
	let prefix = format!(".{file_name}.");
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if name.starts_with(&prefix)
			&& name.ends_with(".download")
			&& std::fs::remove_file(entry.path()).is_ok()
		{
			tracing::info!(
				path = %entry.path().display(),
				"removed a partial download left by an interrupted run"
			);
		}
	}
}

/// Exclusive per-binary lock file held for the duration of a download.
struct DownloadLock {
	file: File,
}

impl Drop for DownloadLock {
	fn drop(&mut self) {
		let _ = self.file.unlock();
	}
}

/// Whether a lock error means the filesystem cannot lock files at all, for
/// example some network or FUSE mounts, rather than a real I/O failure.
pub(crate) fn lock_unsupported(error: &std::io::Error) -> bool {
	if error.kind() == std::io::ErrorKind::Unsupported {
		return true;
	}
	#[cfg(unix)]
	{
		matches!(
			error.raw_os_error(),
			Some(libc::ENOLCK | libc::EOPNOTSUPP | libc::ENOSYS)
		) || error.raw_os_error() == Some(libc::ENOTSUP)
	}
	#[cfg(not(unix))]
	{
		false
	}
}

/// Takes the per-binary download lock. Returns `None` when the filesystem does
/// not support locking, in which case the download proceeds unlocked.
async fn acquire_lock(
	binary: &ReleaseBinary,
	lock_path: &Path,
	timeout: Duration,
) -> Result<Option<DownloadLock>> {
	let file = OpenOptions::new()
		.create(true)
		.truncate(false)
		.write(true)
		.open(lock_path)
		.map_err(|error| io_error(binary, "open lock file", lock_path, error))?;

	let deadline = tokio::time::Instant::now() + timeout;
	let mut retry = tokio::time::interval_at(
		tokio::time::Instant::now() + LOCK_RETRY_INTERVAL,
		LOCK_RETRY_INTERVAL,
	);
	retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	let mut logged_wait = false;
	loop {
		match file.try_lock() {
			Ok(()) => return Ok(Some(DownloadLock { file })),
			Err(std::fs::TryLockError::WouldBlock) => {}
			Err(std::fs::TryLockError::Error(error)) if lock_unsupported(&error) => {
				tracing::warn!(
					binary = binary.display_name,
					version = %binary.version,
					lock_path = %lock_path.display(),
					%error,
					hint = %format!("set {CACHE_DIR_ENV} to a directory on a local filesystem"),
					"file locking is not supported in the binary cache directory, so parallel first runs may each download the binary"
				);
				return Ok(None);
			}
			Err(std::fs::TryLockError::Error(error)) => {
				return Err(io_error(binary, "lock", lock_path, error));
			}
		}

		if !logged_wait {
			logged_wait = true;
			tracing::info!(
				binary = binary.display_name,
				version = %binary.version,
				lock_path = %lock_path.display(),
				timeout_secs = timeout.as_secs(),
				"waiting for another process to finish downloading this binary"
			);
		}
		if tokio::time::Instant::now() >= deadline {
			return Err(BinaryDownloadError::LockTimeout {
				binary: binary.display_name.to_owned(),
				version: binary.version.clone(),
				lock_path: lock_path.display().to_string(),
				timeout_secs: timeout.as_secs(),
				workaround: binary.workaround(),
			}
			.build());
		}
		retry.tick().await;
	}
}

async fn download_verified(
	binary: &ReleaseBinary,
	destination: &Path,
	reason: &'static str,
) -> Result<()> {
	let client = Client::builder()
		.connect_timeout(CONNECT_TIMEOUT)
		.build()
		.map_err(|error| request_error(binary, &binary.manifest_url(), error))?;

	let url = binary.artifact_url();
	let host = Url::parse(&url)
		.ok()
		.and_then(|url| url.host_str().map(str::to_owned))
		.unwrap_or_default();
	tracing::info!(
		binary = binary.display_name,
		version = %binary.version,
		platform = %binary.platform,
		source = %host,
		reason,
		%url,
		"downloading binary (first run only, cached afterwards)"
	);
	let started = Instant::now();

	let manifest_url = binary.manifest_url();
	let client = &client;
	let manifest = with_retries(binary, &manifest_url, |_| {
		fetch_manifest(client, binary, &manifest_url)
	})
	.await?;
	let expected = checksum_for_artifact(&manifest, &binary.artifact).ok_or_else(|| {
		BinaryDownloadError::ChecksumMissing {
			binary: binary.display_name.to_owned(),
			version: binary.version.clone(),
			url: manifest_url.clone(),
			artifact: binary.artifact.clone(),
			workaround: binary.workaround(),
		}
		.build()
	})?;

	let (mut temp, size) = with_retries(binary, &url, |attempt| {
		download_attempt(client, binary, &url, destination, &expected, attempt)
	})
	.await?;

	// Remove the old record first so a replaced binary is never paired with a
	// stale size, then record the verified size only after the rename.
	let record_path = checksum_record_path(destination);
	match std::fs::remove_file(&record_path) {
		Ok(()) => {}
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
		Err(error) => return Err(io_error(binary, "remove", &record_path, error)),
	}
	make_executable(&temp.path)
		.and_then(|()| std::fs::rename(&temp.path, destination))
		.map_err(|error| io_error(binary, "install", destination, error))?;
	temp.installed = true;
	write_checksum_record(&record_path, &expected, size)
		.map_err(|error| io_error(binary, "write", &record_path, error))?;
	sync_directory(destination);

	tracing::info!(
		binary = binary.display_name,
		version = %binary.version,
		platform = %binary.platform,
		path = %destination.display(),
		size_mb = bytes_to_mb(size),
		elapsed_secs = round_tenths(started.elapsed().as_secs_f64()),
		"downloaded and verified binary"
	);
	Ok(())
}

/// One attempt at downloading the artifact into a verified temp file.
async fn download_attempt(
	client: &Client,
	binary: &ReleaseBinary,
	url: &str,
	destination: &Path,
	expected: &str,
	attempt: u32,
) -> std::result::Result<(TempDownload, u64), AttemptError> {
	// The body has its own per-chunk timeout, so bound only the wait for the
	// response headers here instead of the whole transfer.
	let mut response = match tokio::time::timeout(CHUNK_TIMEOUT, client.get(url).send()).await {
		Ok(response) => response.map_err(|error| request_error(binary, url, error))?,
		Err(_) => {
			return Err(request_error(
				binary,
				url,
				format!("no response received for {}s", CHUNK_TIMEOUT.as_secs()),
			)
			.into());
		}
	};
	check_status(binary, url, &response)?;
	let total_bytes = response.content_length();
	tracing::info!(
		binary = binary.display_name,
		version = %binary.version,
		attempt,
		size_mb = total_bytes.map(bytes_to_mb),
		"download started"
	);

	let temp = TempDownload {
		path: temp_download_path(destination, &binary.file_name),
		installed: false,
	};
	let size = stream_to_file(
		binary,
		url,
		&mut response,
		&temp.path,
		total_bytes,
		expected,
	)
	.await?;
	Ok((temp, size))
}

/// A failed download attempt plus the `Retry-After` delay the server asked for.
pub(crate) struct AttemptError {
	pub(crate) error: anyhow::Error,
	pub(crate) retry_after: Option<Duration>,
}

impl From<anyhow::Error> for AttemptError {
	fn from(error: anyhow::Error) -> Self {
		Self {
			error,
			retry_after: None,
		}
	}
}

/// Runs `attempt` with up to `DOWNLOAD_RETRIES` retries and exponential
/// backoff for transient failures. Missing releases and checksum mismatches
/// fail immediately because retrying cannot fix them.
async fn with_retries<T, F, Fut>(binary: &ReleaseBinary, url: &str, mut attempt: F) -> Result<T>
where
	F: FnMut(u32) -> Fut,
	Fut: std::future::Future<Output = std::result::Result<T, AttemptError>>,
{
	let mut delay = RETRY_BASE_DELAY;
	let mut number = 1;
	loop {
		match attempt(number).await {
			Ok(value) => return Ok(value),
			Err(AttemptError { error, retry_after }) if is_retryable(&error) && number <= DOWNLOAD_RETRIES => {
				let wait = retry_after.unwrap_or(delay);
				tracing::warn!(
					binary = binary.display_name,
					version = %binary.version,
					%url,
					attempt = number,
					max_attempts = DOWNLOAD_RETRIES + 1,
					retry_in_secs = wait.as_secs_f64(),
					%error,
					"binary download failed with a transient error, retrying"
				);
				tokio::time::sleep(wait).await;
				delay *= 2;
				number += 1;
			}
			Err(AttemptError { error, .. }) => {
				if is_retryable(&error) {
					tracing::warn!(
						binary = binary.display_name,
						version = %binary.version,
						%url,
						attempts = number,
						"binary download retries exhausted"
					);
				}
				return Err(error);
			}
		}
	}
}

/// Whether a download error is transient: a connection failure, a timeout, an
/// interrupted body, rate limiting, or a server error.
pub(crate) fn is_retryable(error: &anyhow::Error) -> bool {
	let error = rivet_error::RivetError::extract(error);
	if error.group() != "binary_download" {
		return false;
	}
	match error.code() {
		"request_failed" | "incomplete" => true,
		"http_status" => error
			.metadata()
			.and_then(|metadata| metadata.get("status").and_then(|status| status.as_u64()))
			.is_some_and(|status| status == 429 || status >= 500),
		_ => false,
	}
}

/// The `Retry-After` delay of a rate-limited response, capped at
/// `MAX_RETRY_AFTER`. Only the delay-in-seconds form is supported; the
/// HTTP-date form falls back to exponential backoff.
pub(crate) fn retry_after(status: StatusCode, headers: &reqwest::header::HeaderMap) -> Option<Duration> {
	if status != StatusCode::TOO_MANY_REQUESTS {
		return None;
	}
	let secs: u64 = headers
		.get(reqwest::header::RETRY_AFTER)?
		.to_str()
		.ok()?
		.trim()
		.parse()
		.ok()?;
	Some(Duration::from_secs(secs).min(MAX_RETRY_AFTER))
}

/// Streams the response body to `temp_path` while hashing it, then checks the
/// length and checksum. Returns the number of bytes written.
async fn stream_to_file(
	binary: &ReleaseBinary,
	url: &str,
	response: &mut reqwest::Response,
	temp_path: &Path,
	total_bytes: Option<u64>,
	expected: &str,
) -> Result<u64> {
	let mut file =
		File::create(temp_path).map_err(|error| io_error(binary, "create", temp_path, error))?;
	let mut hasher = Sha256::new();
	let mut received: u64 = 0;
	let started = Instant::now();
	let mut last_progress = started;
	let mut hinted = false;

	loop {
		let chunk = match tokio::time::timeout(CHUNK_TIMEOUT, response.chunk()).await {
			Ok(Ok(Some(chunk))) => chunk,
			Ok(Ok(None)) => break,
			Ok(Err(error)) => {
				return Err(incomplete_or_request_error(
					binary,
					url,
					received,
					total_bytes,
					error.to_string(),
				));
			}
			Err(_) => {
				return Err(incomplete_or_request_error(
					binary,
					url,
					received,
					total_bytes,
					format!("no data received for {}s", CHUNK_TIMEOUT.as_secs()),
				));
			}
		};
		hasher.update(&chunk);
		file.write_all(&chunk)
			.map_err(|error| io_error(binary, "write", temp_path, error))?;
		received += chunk.len() as u64;

		if last_progress.elapsed() >= PROGRESS_INTERVAL {
			last_progress = Instant::now();
			log_progress(binary, received, total_bytes, started.elapsed());
			if !hinted {
				hinted = true;
				tracing::info!(
					binary = binary.display_name,
					hint = %binary.skip_hint(),
					"this download happens once per version; to skip it on other machines or containers"
				);
			}
		}
	}

	if let Some(expected_bytes) = total_bytes
		&& received != expected_bytes
	{
		return Err(BinaryDownloadError::Incomplete {
			binary: binary.display_name.to_owned(),
			version: binary.version.clone(),
			url: url.to_owned(),
			received_bytes: received,
			expected_bytes,
			workaround: binary.workaround(),
		}
		.build());
	}

	file.sync_all()
		.map_err(|error| io_error(binary, "flush", temp_path, error))?;
	drop(file);

	let received_hash = hex(&hasher.finalize());
	if !received_hash.eq_ignore_ascii_case(expected) {
		return Err(BinaryDownloadError::ChecksumMismatch {
			binary: binary.display_name.to_owned(),
			version: binary.version.clone(),
			url: url.to_owned(),
			expected: expected.to_owned(),
			received: received_hash,
			workaround: binary.workaround(),
		}
		.build());
	}

	Ok(received)
}

pub(crate) fn write_checksum_record(
	path: &Path,
	checksum: &str,
	size: u64,
) -> std::io::Result<()> {
	let mut file = File::create(path)?;
	file.write_all(format!("{}  {size}\n", checksum.to_ascii_lowercase()).as_bytes())?;
	file.sync_all()
}

/// Flushes the directory entry for a renamed file so the install survives a
/// crash. Best effort, because some platforms cannot open directories.
fn sync_directory(path: &Path) {
	#[cfg(unix)]
	if let Some(directory) = path.parent()
		&& let Err(error) = File::open(directory).and_then(|directory| directory.sync_all())
	{
		tracing::debug!(
			path = %directory.display(),
			%error,
			"could not sync binary cache directory"
		);
	}
	#[cfg(not(unix))]
	{
		let _ = path;
	}
}

fn log_progress(binary: &ReleaseBinary, received: u64, total: Option<u64>, elapsed: Duration) {
	let rate = received as f64 / elapsed.as_secs_f64().max(0.001);
	let percent = total
		.filter(|total| *total > 0)
		.map(|total| (received as f64 / total as f64 * 100.0).round());
	let eta_secs = total
		.filter(|total| *total > received && rate > 0.0)
		.map(|total| ((total - received) as f64 / rate).round());
	tracing::info!(
		binary = binary.display_name,
		version = %binary.version,
		percent,
		downloaded_mb = bytes_to_mb(received),
		total_mb = total.map(bytes_to_mb),
		speed_mb_per_sec = round_tenths(rate / 1_048_576.0),
		eta_secs,
		"still downloading binary"
	);
}

async fn fetch_manifest(
	client: &Client,
	binary: &ReleaseBinary,
	url: &str,
) -> std::result::Result<String, AttemptError> {
	let response = client
		.get(url)
		.timeout(MANIFEST_TIMEOUT)
		.send()
		.await
		.map_err(|error| request_error(binary, url, error))?;
	check_status(binary, url, &response)?;
	Ok(response
		.text()
		.await
		.map_err(|error| request_error(binary, url, error))?)
}

fn check_status(
	binary: &ReleaseBinary,
	url: &str,
	response: &reqwest::Response,
) -> std::result::Result<(), AttemptError> {
	let status = response.status();
	check_status_code(binary, url, status).map_err(|error| AttemptError {
		error,
		retry_after: retry_after(status, response.headers()),
	})
}

pub(crate) fn check_status_code(binary: &ReleaseBinary, url: &str, status: StatusCode) -> Result<()> {
	if status.is_success() {
		return Ok(());
	}
	Err(BinaryDownloadError::HttpStatus {
		binary: binary.display_name.to_owned(),
		version: binary.version.clone(),
		url: url.to_owned(),
		status: status.as_u16(),
		workaround: binary.workaround(),
	}
	.build())
}

/// Parses a `sha256sum` manifest and returns the checksum for `artifact`.
pub fn checksum_for_artifact(manifest: &str, artifact: &str) -> Option<String> {
	manifest.lines().find_map(|line| {
		let mut parts = line.split_whitespace();
		let checksum = parts.next()?;
		let name = parts.next()?.trim_start_matches('*');
		if checksum.len() == 64 && name == artifact {
			Some(checksum.to_owned())
		} else {
			None
		}
	})
}

/// Lowercase hex encoding of a SHA-256 digest.
pub fn sha256_hex(bytes: &[u8]) -> String {
	hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
	use std::fmt::Write as _;
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		let _ = write!(&mut out, "{byte:02x}");
	}
	out
}

fn bytes_to_mb(bytes: u64) -> f64 {
	round_tenths(bytes as f64 / 1_048_576.0)
}

fn round_tenths(value: f64) -> f64 {
	(value * 10.0).round() / 10.0
}

fn request_error(binary: &ReleaseBinary, url: &str, error: impl std::fmt::Display) -> anyhow::Error {
	BinaryDownloadError::RequestFailed {
		binary: binary.display_name.to_owned(),
		version: binary.version.clone(),
		url: url.to_owned(),
		reason: error.to_string(),
		workaround: binary.workaround(),
	}
	.build()
}

fn incomplete_or_request_error(
	binary: &ReleaseBinary,
	url: &str,
	received: u64,
	total: Option<u64>,
	reason: String,
) -> anyhow::Error {
	match total {
		Some(expected_bytes) => BinaryDownloadError::Incomplete {
			binary: binary.display_name.to_owned(),
			version: binary.version.clone(),
			url: url.to_owned(),
			received_bytes: received,
			expected_bytes,
			workaround: binary.workaround(),
		}
		.build(),
		None => request_error(binary, url, reason),
	}
}

fn io_error(
	binary: &ReleaseBinary,
	operation: &str,
	path: &Path,
	error: impl std::fmt::Display,
) -> anyhow::Error {
	BinaryDownloadError::Io {
		binary: binary.display_name.to_owned(),
		version: binary.version.clone(),
		operation: operation.to_owned(),
		path: path.display().to_string(),
		reason: error.to_string(),
		cache_env: CACHE_DIR_ENV.to_owned(),
		workaround: binary.workaround(),
	}
	.build()
}

fn make_executable(path: &Path) -> std::io::Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut permissions = std::fs::metadata(path)?.permissions();
		permissions.set_mode(0o755);
		std::fs::set_permissions(path, permissions)?;
	}
	#[cfg(not(unix))]
	{
		let _ = path;
	}
	Ok(())
}

/// Returns `UnsupportedPlatform` for binaries not published for this host.
pub fn unsupported_platform(
	display_name: &str,
	override_env: &str,
	alternative: &str,
) -> anyhow::Error {
	BinaryDownloadError::UnsupportedPlatform {
		binary: display_name.to_owned(),
		os: std::env::consts::OS.to_owned(),
		arch: std::env::consts::ARCH.to_owned(),
		override_env: override_env.to_owned(),
		alternative: alternative.to_owned(),
	}
	.build()
}

#[cfg(test)]
#[path = "../tests/unit/download.rs"]
mod tests;
