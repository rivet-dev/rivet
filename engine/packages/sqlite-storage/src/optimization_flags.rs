//! Central SQLite optimization feature flags.

use std::{env, sync::OnceLock};

pub const READ_AHEAD_ENV: &str = "RIVETKIT_SQLITE_OPT_READ_AHEAD";
pub const CACHE_HIT_PREDICTOR_TRAINING_ENV: &str =
	"RIVETKIT_SQLITE_OPT_CACHE_HIT_PREDICTOR_TRAINING";
pub const RECENT_PAGE_HINTS_ENV: &str = "RIVETKIT_SQLITE_OPT_RECENT_PAGE_HINTS";
pub const ADAPTIVE_READ_AHEAD_ENV: &str = "RIVETKIT_SQLITE_OPT_ADAPTIVE_READ_AHEAD";
pub const PRELOAD_HINT_FLUSH_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_FLUSH";
pub const PRELOAD_HINTS_ON_OPEN_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINTS_ON_OPEN";
pub const PRELOAD_HINT_HOT_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_HOT_PAGES";
pub const PRELOAD_HINT_EARLY_PAGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_EARLY_PAGES";
pub const PRELOAD_HINT_SCAN_RANGES_ENV: &str = "RIVETKIT_SQLITE_OPT_PRELOAD_HINT_SCAN_RANGES";
pub const DEDUP_GET_PAGES_META_ENV: &str = "RIVETKIT_SQLITE_OPT_DEDUP_GET_PAGES_META";
pub const CACHE_GET_PAGES_VALIDATION_ENV: &str = "RIVETKIT_SQLITE_OPT_CACHE_GET_PAGES_VALIDATION";
pub const RANGE_READS_ENV: &str = "RIVETKIT_SQLITE_OPT_RANGE_READS";
pub const BATCH_CHUNK_READS_ENV: &str = "RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteOptimizationFlags {
	pub read_ahead: bool,
	pub cache_hit_predictor_training: bool,
	pub recent_page_hints: bool,
	pub adaptive_read_ahead: bool,
	pub preload_hint_flush: bool,
	pub preload_hints_on_open: bool,
	pub preload_hint_hot_pages: bool,
	pub preload_hint_early_pages: bool,
	pub preload_hint_scan_ranges: bool,
	pub dedup_get_pages_meta: bool,
	pub cache_get_pages_validation: bool,
	pub range_reads: bool,
	pub batch_chunk_reads: bool,
}

impl Default for SqliteOptimizationFlags {
	fn default() -> Self {
		Self {
			read_ahead: true,
			cache_hit_predictor_training: true,
			recent_page_hints: true,
			adaptive_read_ahead: true,
			preload_hint_flush: true,
			preload_hints_on_open: true,
			preload_hint_hot_pages: true,
			preload_hint_early_pages: true,
			preload_hint_scan_ranges: true,
			dedup_get_pages_meta: true,
			cache_get_pages_validation: true,
			range_reads: true,
			batch_chunk_reads: true,
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
			dedup_get_pages_meta: enabled_by_default(read_env(DEDUP_GET_PAGES_META_ENV).as_deref()),
			cache_get_pages_validation: enabled_by_default(
				read_env(CACHE_GET_PAGES_VALIDATION_ENV).as_deref(),
			),
			range_reads: enabled_by_default(read_env(RANGE_READS_ENV).as_deref()),
			batch_chunk_reads: enabled_by_default(read_env(BATCH_CHUNK_READS_ENV).as_deref()),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flags_default_enabled_and_explicitly_disableable() {
		let flags = SqliteOptimizationFlags::from_env_reader(|key| match key {
			READ_AHEAD_ENV => Some("false".to_string()),
			RECENT_PAGE_HINTS_ENV => Some("0".to_string()),
			PRELOAD_HINT_SCAN_RANGES_ENV => Some("disabled".to_string()),
			CACHE_GET_PAGES_VALIDATION_ENV => Some("off".to_string()),
			BATCH_CHUNK_READS_ENV => Some("no".to_string()),
			_ => None,
		});

		assert!(!flags.read_ahead);
		assert!(flags.cache_hit_predictor_training);
		assert!(!flags.recent_page_hints);
		assert!(flags.preload_hint_hot_pages);
		assert!(flags.preload_hint_early_pages);
		assert!(!flags.preload_hint_scan_ranges);
		assert!(!flags.cache_get_pages_validation);
		assert!(flags.range_reads);
		assert!(!flags.batch_chunk_reads);
	}
}
