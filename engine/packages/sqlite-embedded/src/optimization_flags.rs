//! Process page budgets and I/O concurrency. Prefetch policy is owned by the server.

use std::{env, sync::OnceLock};

pub const VFS_PAGE_CACHE_MODE_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_MODE";
pub const VFS_PAGE_CACHE_CAPACITY_PAGES_ENV: &str =
	"RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_CAPACITY_PAGES";
pub const VFS_STAGING_CACHE_TTL_MS_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_STAGING_CACHE_TTL_MS";
pub const PAGER_CACHE_SIZE_KIB_ENV: &str = "RIVETKIT_SQLITE_OPT_PAGER_CACHE_SIZE_KIB";
pub const PAGER_PROCESS_CACHE_KIB_ENV: &str = "RIVETKIT_SQLITE_OPT_PAGER_PROCESS_CACHE_KIB";
pub const IDLE_RELEASE_MS_ENV: &str = "RIVETKIT_SQLITE_OPT_IDLE_RELEASE_MS";
pub const PAGE_RANGE_CONCURRENCY_ENV: &str = "RIVETKIT_SQLITE_OPT_PAGE_RANGE_CONCURRENCY";
pub const FLUSH_RETRY_DEADLINE_MS_ENV: &str = "RIVETKIT_SQLITE_OPT_FLUSH_RETRY_DEADLINE_MS";
pub const FLUSH_RETRY_BACKOFF_MIN_MS_ENV: &str = "RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MIN_MS";
pub const FLUSH_RETRY_BACKOFF_MAX_MS_ENV: &str = "RIVETKIT_SQLITE_OPT_FLUSH_RETRY_BACKOFF_MAX_MS";
pub const DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES: u64 = 131_072;
pub const MAX_VFS_PAGE_CACHE_CAPACITY_PAGES: u64 = 4_194_304;
pub const DEFAULT_VFS_STAGING_CACHE_TTL_MS: u64 = 30_000;
pub const MAX_VFS_STAGING_CACHE_TTL_MS: u64 = 300_000;
pub const DEFAULT_PAGER_CACHE_SIZE_KIB: u64 = 8 * 1024;
pub const MAX_PAGER_CACHE_SIZE_KIB: u64 = 256 * 1024;
pub const DEFAULT_PAGER_PROCESS_CACHE_KIB: u64 = 256 * 1024;
pub const MAX_PAGER_PROCESS_CACHE_KIB: u64 = 16 * 1024 * 1024;
pub const DEFAULT_IDLE_RELEASE_MS: u64 = 30_000;
pub const MAX_IDLE_RELEASE_MS: u64 = 3_600_000;
pub const DEFAULT_PAGE_RANGE_CONCURRENCY: usize = 16;
pub const MAX_PAGE_RANGE_CONCURRENCY: usize = 64;
pub const DEFAULT_FLUSH_RETRY_DEADLINE_MS: u64 = 30_000;
pub const MAX_FLUSH_RETRY_DEADLINE_MS: u64 = 600_000;
pub const DEFAULT_FLUSH_RETRY_BACKOFF_MIN_MS: u64 = 50;
pub const DEFAULT_FLUSH_RETRY_BACKOFF_MAX_MS: u64 = 2_000;
pub const MAX_FLUSH_RETRY_BACKOFF_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteVfsPageCacheMode {
	Off,
	Target,
	Startup,
	Prefetch,
	All,
}

impl SqliteVfsPageCacheMode {
	pub fn caches_any_pages(self) -> bool {
		!matches!(self, Self::Off)
	}

	pub fn caches_target_pages(self) -> bool {
		matches!(
			self,
			Self::Target | Self::Startup | Self::Prefetch | Self::All
		)
	}

	pub fn caches_prefetched_pages(self) -> bool {
		matches!(self, Self::Prefetch | Self::All)
	}

	pub fn caches_startup_preloaded_pages(self) -> bool {
		matches!(self, Self::Startup | Self::All)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteOptimizationFlags {
	pub vfs_page_cache_mode: SqliteVfsPageCacheMode,
	pub vfs_page_buffer_process_pages: u64,
	pub vfs_staging_cache_ttl_ms: u64,
	pub pager_cache_size_kib: u64,
	pub pager_process_cache_kib: u64,
	pub idle_release_ms: u64,
	pub page_range_concurrency: usize,
	/// How long a deferred-commit flush retries before the database breaks.
	pub flush_retry_deadline_ms: u64,
	pub flush_retry_backoff_min_ms: u64,
	pub flush_retry_backoff_max_ms: u64,
}
impl Default for SqliteOptimizationFlags {
	fn default() -> Self {
		Self::from_env_reader(|_| None)
	}
}
impl SqliteOptimizationFlags {
	fn from_process_env() -> Self {
		Self::from_env_reader(|key| env::var(key).ok())
	}
	pub fn from_env_reader(mut read_env: impl FnMut(&str) -> Option<String>) -> Self {
		Self {
			vfs_page_cache_mode: vfs_page_cache_mode_by_default(
				read_env(VFS_PAGE_CACHE_MODE_ENV).as_deref(),
				SqliteVfsPageCacheMode::All,
			),
			vfs_page_buffer_process_pages: u64_bounded_by_default(
				read_env(VFS_PAGE_CACHE_CAPACITY_PAGES_ENV).as_deref(),
				DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES,
				MAX_VFS_PAGE_CACHE_CAPACITY_PAGES,
			),
			vfs_staging_cache_ttl_ms: u64_bounded_by_default(
				read_env(VFS_STAGING_CACHE_TTL_MS_ENV).as_deref(),
				DEFAULT_VFS_STAGING_CACHE_TTL_MS,
				MAX_VFS_STAGING_CACHE_TTL_MS,
			),
			pager_cache_size_kib: u64_bounded_by_default(
				read_env(PAGER_CACHE_SIZE_KIB_ENV).as_deref(),
				DEFAULT_PAGER_CACHE_SIZE_KIB,
				MAX_PAGER_CACHE_SIZE_KIB,
			),
			pager_process_cache_kib: u64_bounded_by_default(
				read_env(PAGER_PROCESS_CACHE_KIB_ENV).as_deref(),
				DEFAULT_PAGER_PROCESS_CACHE_KIB,
				MAX_PAGER_PROCESS_CACHE_KIB,
			),
			idle_release_ms: u64_bounded_by_default(
				read_env(IDLE_RELEASE_MS_ENV).as_deref(),
				DEFAULT_IDLE_RELEASE_MS,
				MAX_IDLE_RELEASE_MS,
			),
			page_range_concurrency: usize_bounded_by_default(
				read_env(PAGE_RANGE_CONCURRENCY_ENV).as_deref(),
				DEFAULT_PAGE_RANGE_CONCURRENCY,
				MAX_PAGE_RANGE_CONCURRENCY,
			)
			.max(1),
			flush_retry_deadline_ms: u64_bounded_by_default(
				read_env(FLUSH_RETRY_DEADLINE_MS_ENV).as_deref(),
				DEFAULT_FLUSH_RETRY_DEADLINE_MS,
				MAX_FLUSH_RETRY_DEADLINE_MS,
			),
			flush_retry_backoff_min_ms: u64_bounded_by_default(
				read_env(FLUSH_RETRY_BACKOFF_MIN_MS_ENV).as_deref(),
				DEFAULT_FLUSH_RETRY_BACKOFF_MIN_MS,
				MAX_FLUSH_RETRY_BACKOFF_MS,
			),
			flush_retry_backoff_max_ms: u64_bounded_by_default(
				read_env(FLUSH_RETRY_BACKOFF_MAX_MS_ENV).as_deref(),
				DEFAULT_FLUSH_RETRY_BACKOFF_MAX_MS,
				MAX_FLUSH_RETRY_BACKOFF_MS,
			),
		}
	}
}
pub fn sqlite_optimization_flags() -> &'static SqliteOptimizationFlags {
	static FLAGS: OnceLock<SqliteOptimizationFlags> = OnceLock::new();
	FLAGS.get_or_init(SqliteOptimizationFlags::from_process_env)
}

fn usize_bounded_by_default(value: Option<&str>, default: usize, max: usize) -> usize {
	value
		.and_then(|value| value.trim().parse::<usize>().ok())
		.unwrap_or(default)
		.min(max)
}

fn u64_bounded_by_default(value: Option<&str>, default: u64, max: u64) -> u64 {
	value
		.and_then(|value| value.trim().parse::<u64>().ok())
		.unwrap_or(default)
		.min(max)
}

fn vfs_page_cache_mode_by_default(
	value: Option<&str>,
	default: SqliteVfsPageCacheMode,
) -> SqliteVfsPageCacheMode {
	match value.map(|value| value.trim().to_ascii_lowercase()) {
		Some(value)
			if matches!(
				value.as_str(),
				"off" | "0" | "false" | "no" | "disabled" | "disable"
			) =>
		{
			SqliteVfsPageCacheMode::Off
		}
		Some(value) if value == "target" => SqliteVfsPageCacheMode::Target,
		Some(value) if value == "startup" => SqliteVfsPageCacheMode::Startup,
		Some(value) if value == "prefetch" => SqliteVfsPageCacheMode::Prefetch,
		Some(value) if value == "all" || value == "on" || value == "true" || value == "1" => {
			SqliteVfsPageCacheMode::All
		}
		_ => default,
	}
}


