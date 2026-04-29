//! Central SQLite optimization feature flags.

use std::{env, sync::OnceLock};

pub const READ_AHEAD_ENV: &str = "RIVETKIT_SQLITE_OPT_READ_AHEAD";
pub const CACHE_HIT_PREDICTOR_TRAINING_ENV: &str =
	"RIVETKIT_SQLITE_OPT_CACHE_HIT_PREDICTOR_TRAINING";
pub const RECENT_PAGE_HINTS_ENV: &str = "RIVETKIT_SQLITE_OPT_RECENT_PAGE_HINTS";
pub const ADAPTIVE_READ_AHEAD_ENV: &str = "RIVETKIT_SQLITE_OPT_ADAPTIVE_READ_AHEAD";
pub const PRELOAD_HINT_FLUSH_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_FLUSH";
pub const STARTUP_PRELOAD_MAX_BYTES_ENV: &str = "RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_MAX_BYTES";
pub const STARTUP_PRELOAD_FIRST_PAGES_ENV: &str =
	"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_FIRST_PAGES";
pub const STARTUP_PRELOAD_FIRST_PAGE_COUNT_ENV: &str =
	"RIVETKIT_SQLITE_OPT_STARTUP_PRELOAD_FIRST_PAGE_COUNT";
pub const PRELOAD_HINTS_ON_OPEN_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINTS_ON_OPEN";
pub const PRELOAD_HINT_HOT_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_HOT_PAGES";
pub const PRELOAD_HINT_EARLY_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_EARLY_PAGES";
pub const PRELOAD_HINT_SCAN_RANGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_SCAN_RANGES";
pub const CACHE_GET_PAGES_VALIDATION_ENV: &str = "RIVETKIT_SQLITE_OPT_CACHE_GET_PAGES_VALIDATION";
pub const RANGE_READS_ENV: &str = "RIVETKIT_SQLITE_OPT_RANGE_READS";
pub const BATCH_CHUNK_READS_ENV: &str = "RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS";
pub const DECODED_LTX_CACHE_ENV: &str = "RIVETKIT_SQLITE_OPT_DECODED_LTX_CACHE";
pub const VFS_PAGE_CACHE_CAPACITY_PAGES_ENV: &str =
	"RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_CAPACITY_PAGES";
pub const VFS_CACHE_FETCHED_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_CACHE_FETCHED_PAGES";
pub const VFS_CACHE_PREFETCHED_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_CACHE_PREFETCHED_PAGES";
pub const VFS_CACHE_STARTUP_PRELOADED_PAGES_ENV: &str =
	"RIVETKIT_SQLITE_OPT_VFS_CACHE_STARTUP_PRELOADED_PAGES";
pub const VFS_SCAN_RESISTANT_CACHE_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_SCAN_RESISTANT_CACHE";
pub const VFS_PROTECTED_CACHE_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_VFS_PROTECTED_CACHE_PAGES";

pub const DEFAULT_STARTUP_PRELOAD_MAX_BYTES: usize = 1024 * 1024;
pub const MAX_STARTUP_PRELOAD_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const DEFAULT_STARTUP_PRELOAD_FIRST_PAGE_COUNT: u32 = 1;
pub const MAX_STARTUP_PRELOAD_FIRST_PAGE_COUNT: u32 = 256;
pub const DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES: u64 = 50_000;
pub const MIN_VFS_PAGE_CACHE_CAPACITY_PAGES: u64 = 1;
pub const MAX_VFS_PAGE_CACHE_CAPACITY_PAGES: u64 = 500_000;
pub const DEFAULT_VFS_PROTECTED_CACHE_PAGES: usize = 512;
pub const MAX_VFS_PROTECTED_CACHE_PAGES: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteOptimizationFlags {
	pub read_ahead: bool,
	pub cache_hit_predictor_training: bool,
	pub recent_page_hints: bool,
	pub adaptive_read_ahead: bool,
	pub preload_hint_flush: bool,
	pub startup_preload_max_bytes: usize,
	pub startup_preload_first_pages: bool,
	pub startup_preload_first_page_count: u32,
	pub preload_hints_on_open: bool,
	pub preload_hint_hot_pages: bool,
	pub preload_hint_early_pages: bool,
	pub preload_hint_scan_ranges: bool,
	pub cache_get_pages_validation: bool,
	pub range_reads: bool,
	pub batch_chunk_reads: bool,
	pub decoded_ltx_cache: bool,
	pub vfs_page_cache_capacity_pages: u64,
	pub vfs_cache_fetched_pages: bool,
	pub vfs_cache_prefetched_pages: bool,
	pub vfs_cache_startup_preloaded_pages: bool,
	pub vfs_scan_resistant_cache: bool,
	pub vfs_protected_cache_pages: usize,
}

impl Default for SqliteOptimizationFlags {
	fn default() -> Self {
		Self {
			read_ahead: true,
			cache_hit_predictor_training: true,
			recent_page_hints: true,
			adaptive_read_ahead: true,
			preload_hint_flush: true,
			startup_preload_max_bytes: DEFAULT_STARTUP_PRELOAD_MAX_BYTES,
			startup_preload_first_pages: true,
			startup_preload_first_page_count: DEFAULT_STARTUP_PRELOAD_FIRST_PAGE_COUNT,
			preload_hints_on_open: true,
			preload_hint_hot_pages: true,
			preload_hint_early_pages: true,
			preload_hint_scan_ranges: true,
			cache_get_pages_validation: true,
			range_reads: true,
			batch_chunk_reads: true,
			decoded_ltx_cache: true,
			vfs_page_cache_capacity_pages: DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES,
			vfs_cache_fetched_pages: true,
			vfs_cache_prefetched_pages: true,
			vfs_cache_startup_preloaded_pages: true,
			vfs_scan_resistant_cache: true,
			vfs_protected_cache_pages: DEFAULT_VFS_PROTECTED_CACHE_PAGES,
		}
	}
}

impl SqliteOptimizationFlags {
	fn from_process_env() -> Self {
		Self::from_env_reader(|key| env::var(key).ok())
	}

	pub fn from_env_reader(mut read_env: impl FnMut(&str) -> Option<String>) -> Self {
		Self {
			read_ahead: enabled_by_default(read_env(READ_AHEAD_ENV).as_deref()),
			cache_hit_predictor_training: enabled_by_default(
				read_env(CACHE_HIT_PREDICTOR_TRAINING_ENV).as_deref(),
			),
			recent_page_hints: enabled_by_default(read_env(RECENT_PAGE_HINTS_ENV).as_deref()),
			adaptive_read_ahead: enabled_by_default(read_env(ADAPTIVE_READ_AHEAD_ENV).as_deref()),
			preload_hint_flush: enabled_by_default(read_env(PRELOAD_HINT_FLUSH_ENV).as_deref()),
			startup_preload_max_bytes: usize_bounded_by_default(
				read_env(STARTUP_PRELOAD_MAX_BYTES_ENV).as_deref(),
				DEFAULT_STARTUP_PRELOAD_MAX_BYTES,
				MAX_STARTUP_PRELOAD_MAX_BYTES,
			),
			startup_preload_first_pages: enabled_by_default(
				read_env(STARTUP_PRELOAD_FIRST_PAGES_ENV).as_deref(),
			),
			startup_preload_first_page_count: u32_bounded_by_default(
				read_env(STARTUP_PRELOAD_FIRST_PAGE_COUNT_ENV).as_deref(),
				DEFAULT_STARTUP_PRELOAD_FIRST_PAGE_COUNT,
				MAX_STARTUP_PRELOAD_FIRST_PAGE_COUNT,
			),
			preload_hints_on_open: enabled_by_default(read_env(PRELOAD_HINTS_ON_OPEN_ENV).as_deref()),
			preload_hint_hot_pages: enabled_by_default(
				read_env(PRELOAD_HINT_HOT_PAGES_ENV).as_deref(),
			),
			preload_hint_early_pages: enabled_by_default(
				read_env(PRELOAD_HINT_EARLY_PAGES_ENV).as_deref(),
			),
			preload_hint_scan_ranges: enabled_by_default(
				read_env(PRELOAD_HINT_SCAN_RANGES_ENV).as_deref(),
			),
			cache_get_pages_validation: enabled_by_default(
				read_env(CACHE_GET_PAGES_VALIDATION_ENV).as_deref(),
			),
			range_reads: enabled_by_default(read_env(RANGE_READS_ENV).as_deref()),
			batch_chunk_reads: enabled_by_default(read_env(BATCH_CHUNK_READS_ENV).as_deref()),
			decoded_ltx_cache: enabled_by_default(read_env(DECODED_LTX_CACHE_ENV).as_deref()),
			vfs_page_cache_capacity_pages: u64_bounded_by_default(
				read_env(VFS_PAGE_CACHE_CAPACITY_PAGES_ENV).as_deref(),
				DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES,
				MIN_VFS_PAGE_CACHE_CAPACITY_PAGES,
				MAX_VFS_PAGE_CACHE_CAPACITY_PAGES,
			),
			vfs_cache_fetched_pages: enabled_by_default(
				read_env(VFS_CACHE_FETCHED_PAGES_ENV).as_deref(),
			),
			vfs_cache_prefetched_pages: enabled_by_default(
				read_env(VFS_CACHE_PREFETCHED_PAGES_ENV).as_deref(),
			),
			vfs_cache_startup_preloaded_pages: enabled_by_default(
				read_env(VFS_CACHE_STARTUP_PRELOADED_PAGES_ENV).as_deref(),
			),
			vfs_scan_resistant_cache: enabled_by_default(
				read_env(VFS_SCAN_RESISTANT_CACHE_ENV).as_deref(),
			),
			vfs_protected_cache_pages: usize_bounded_by_default(
				read_env(VFS_PROTECTED_CACHE_PAGES_ENV).as_deref(),
				DEFAULT_VFS_PROTECTED_CACHE_PAGES,
				MAX_VFS_PROTECTED_CACHE_PAGES,
			),
		}
	}
}

pub fn sqlite_optimization_flags() -> &'static SqliteOptimizationFlags {
	static FLAGS: OnceLock<SqliteOptimizationFlags> = OnceLock::new();
	FLAGS.get_or_init(SqliteOptimizationFlags::from_process_env)
}

fn enabled_by_default(value: Option<&str>) -> bool {
	match value.map(|value| value.trim().to_ascii_lowercase()) {
		Some(value)
			if matches!(
				value.as_str(),
				"0" | "false" | "off" | "no" | "disabled" | "disable"
			) =>
		{
			false
		}
		_ => true,
	}
}

fn usize_bounded_by_default(value: Option<&str>, default: usize, max: usize) -> usize {
	value
		.and_then(|value| value.trim().parse::<usize>().ok())
		.unwrap_or(default)
		.min(max)
}

fn u64_bounded_by_default(value: Option<&str>, default: u64, min: u64, max: u64) -> u64 {
	value
		.and_then(|value| value.trim().parse::<u64>().ok())
		.unwrap_or(default)
		.max(min)
		.min(max)
}

fn u32_bounded_by_default(value: Option<&str>, default: u32, max: u32) -> u32 {
	value
		.and_then(|value| value.trim().parse::<u32>().ok())
		.unwrap_or(default)
		.min(max)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flags_default_enabled_and_explicitly_disableable() {
		let flags = SqliteOptimizationFlags::from_env_reader(|key| match key {
			READ_AHEAD_ENV => Some("false".to_string()),
			CACHE_HIT_PREDICTOR_TRAINING_ENV => Some("false".to_string()),
			RECENT_PAGE_HINTS_ENV => Some("0".to_string()),
			ADAPTIVE_READ_AHEAD_ENV => Some("false".to_string()),
			PRELOAD_HINT_FLUSH_ENV => Some("false".to_string()),
			STARTUP_PRELOAD_MAX_BYTES_ENV => Some("0".to_string()),
			STARTUP_PRELOAD_FIRST_PAGES_ENV => Some("false".to_string()),
			STARTUP_PRELOAD_FIRST_PAGE_COUNT_ENV => Some("0".to_string()),
			PRELOAD_HINTS_ON_OPEN_ENV => Some("false".to_string()),
			PRELOAD_HINT_HOT_PAGES_ENV => Some("false".to_string()),
			PRELOAD_HINT_EARLY_PAGES_ENV => Some("false".to_string()),
			PRELOAD_HINT_SCAN_RANGES_ENV => Some("disabled".to_string()),
			CACHE_GET_PAGES_VALIDATION_ENV => Some("off".to_string()),
			RANGE_READS_ENV => Some("false".to_string()),
			BATCH_CHUNK_READS_ENV => Some("no".to_string()),
			DECODED_LTX_CACHE_ENV => Some("disable".to_string()),
			VFS_PAGE_CACHE_CAPACITY_PAGES_ENV => Some("0".to_string()),
			VFS_CACHE_FETCHED_PAGES_ENV => Some("false".to_string()),
			VFS_CACHE_PREFETCHED_PAGES_ENV => Some("false".to_string()),
			VFS_CACHE_STARTUP_PRELOADED_PAGES_ENV => Some("false".to_string()),
			VFS_SCAN_RESISTANT_CACHE_ENV => Some("false".to_string()),
			VFS_PROTECTED_CACHE_PAGES_ENV => Some("0".to_string()),
			_ => None,
		});

		assert!(!flags.read_ahead);
		assert!(!flags.cache_hit_predictor_training);
		assert!(!flags.recent_page_hints);
		assert!(!flags.adaptive_read_ahead);
		assert!(!flags.preload_hint_flush);
		assert_eq!(flags.startup_preload_max_bytes, 0);
		assert!(!flags.startup_preload_first_pages);
		assert_eq!(flags.startup_preload_first_page_count, 0);
		assert!(!flags.preload_hints_on_open);
		assert!(!flags.preload_hint_hot_pages);
		assert!(!flags.preload_hint_early_pages);
		assert!(!flags.preload_hint_scan_ranges);
		assert!(!flags.cache_get_pages_validation);
		assert!(!flags.range_reads);
		assert!(!flags.batch_chunk_reads);
		assert!(!flags.decoded_ltx_cache);
		assert_eq!(
			flags.vfs_page_cache_capacity_pages,
			MIN_VFS_PAGE_CACHE_CAPACITY_PAGES
		);
		assert!(!flags.vfs_cache_fetched_pages);
		assert!(!flags.vfs_cache_prefetched_pages);
		assert!(!flags.vfs_cache_startup_preloaded_pages);
		assert!(!flags.vfs_scan_resistant_cache);
		assert_eq!(flags.vfs_protected_cache_pages, 0);
	}

	#[test]
	fn preload_numeric_config_defaults_and_clamps() {
		let invalid = SqliteOptimizationFlags::from_env_reader(|key| match key {
			STARTUP_PRELOAD_MAX_BYTES_ENV => Some("not-a-number".to_string()),
			STARTUP_PRELOAD_FIRST_PAGE_COUNT_ENV => Some("nope".to_string()),
			VFS_PAGE_CACHE_CAPACITY_PAGES_ENV => Some("invalid".to_string()),
			VFS_PROTECTED_CACHE_PAGES_ENV => Some("invalid".to_string()),
			_ => None,
		});
		assert_eq!(
			invalid.startup_preload_max_bytes,
			DEFAULT_STARTUP_PRELOAD_MAX_BYTES
		);
		assert_eq!(
			invalid.startup_preload_first_page_count,
			DEFAULT_STARTUP_PRELOAD_FIRST_PAGE_COUNT
		);
		assert_eq!(
			invalid.vfs_page_cache_capacity_pages,
			DEFAULT_VFS_PAGE_CACHE_CAPACITY_PAGES
		);
		assert_eq!(
			invalid.vfs_protected_cache_pages,
			DEFAULT_VFS_PROTECTED_CACHE_PAGES
		);

		let clamped = SqliteOptimizationFlags::from_env_reader(|key| match key {
			STARTUP_PRELOAD_MAX_BYTES_ENV => Some((MAX_STARTUP_PRELOAD_MAX_BYTES + 1).to_string()),
			STARTUP_PRELOAD_FIRST_PAGE_COUNT_ENV => {
				Some((MAX_STARTUP_PRELOAD_FIRST_PAGE_COUNT + 1).to_string())
			}
			VFS_PAGE_CACHE_CAPACITY_PAGES_ENV => {
				Some((MAX_VFS_PAGE_CACHE_CAPACITY_PAGES + 1).to_string())
			}
			VFS_PROTECTED_CACHE_PAGES_ENV => Some((MAX_VFS_PROTECTED_CACHE_PAGES + 1).to_string()),
			_ => None,
		});
		assert_eq!(
			clamped.startup_preload_max_bytes,
			MAX_STARTUP_PRELOAD_MAX_BYTES
		);
		assert_eq!(
			clamped.startup_preload_first_page_count,
			MAX_STARTUP_PRELOAD_FIRST_PAGE_COUNT
		);
		assert_eq!(
			clamped.vfs_page_cache_capacity_pages,
			MAX_VFS_PAGE_CACHE_CAPACITY_PAGES
		);
		assert_eq!(
			clamped.vfs_protected_cache_pages,
			MAX_VFS_PROTECTED_CACHE_PAGES
		);
	}
}
