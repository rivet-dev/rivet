//! Disk capacity of a storage worker's log shard and cold-store disk cache.
//!
//! Both live on local filesystems. Each can be configured in bytes. An unset capacity is
//! sized at startup from the free space of its filesystem: when the log and the cache share
//! a filesystem they split one pool, and when they are on separate filesystems each takes
//! its own. A configured capacity that does not fit the free space fails startup, so a
//! worker never accepts a log it cannot store.

use anyhow::{Context, Result, bail, ensure};
use rivet_env::fs::{DiskSpace, Fs};
use std::path::{Path, PathBuf};

/// Share of a shared pool given to the log when neither capacity is configured. The rest
/// goes to the cache, which only saves object-store reads.
pub const AUTO_LOG_SHARE_PERCENT: u64 = 50;

/// Fraction of a filesystem never assigned automatically, for other files and headroom.
pub const RESERVE_PERCENT: u64 = 10;

/// Smallest reserve kept free on a filesystem by automatic sizing.
pub const MIN_RESERVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Name of the default cache directory inside the log directory.
pub const DEFAULT_CACHE_DIRECTORY: &str = "cold-cache";

/// The resolved capacities of one storage worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskBudget {
	pub log_disk_bytes: u64,
	pub cache: Option<DiskCacheBudget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskCacheBudget {
	pub directory: PathBuf,
	pub capacity_bytes: u64,
	pub part_bytes: usize,
}

/// One directory's filesystem, the bytes the directory already holds, and its configured
/// capacity, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
	pub space: DiskSpace,
	pub used_bytes: u64,
	pub configured_bytes: Option<u64>,
}

/// Bytes automatic sizing may assign on `space`, counting what our directories already
/// hold as available because they are ours to reuse.
fn pool_bytes(space: &DiskSpace, used_bytes: u64) -> u64 {
	let reserve = percent_of(space.total_bytes, RESERVE_PERCENT).max(MIN_RESERVE_BYTES);
	space
		.available_bytes
		.saturating_add(used_bytes)
		.saturating_sub(reserve)
}

/// Bytes a configured capacity may use on `space`: the free space plus what our
/// directories already hold. No reserve applies to an operator's explicit choice.
fn usable_bytes(space: &DiskSpace, used_bytes: u64) -> u64 {
	space.available_bytes.saturating_add(used_bytes)
}

/// Resolves the log and cache capacities. `cache` is `None` when the cache is disabled.
/// Returns the log bytes and the cache bytes.
pub fn plan(log: Placement, cache: Option<Placement>) -> Result<(u64, Option<u64>)> {
	let Some(cache) = cache else {
		let log_bytes = match log.configured_bytes {
			Some(bytes) => {
				check_fits("log shard", bytes, usable_bytes(&log.space, log.used_bytes))?;
				bytes
			}
			None => auto("log shard", pool_bytes(&log.space, log.used_bytes))?,
		};
		return Ok((log_bytes, None));
	};

	if log.space.filesystem_id != cache.space.filesystem_id {
		let log_bytes = match log.configured_bytes {
			Some(bytes) => {
				check_fits("log shard", bytes, usable_bytes(&log.space, log.used_bytes))?;
				bytes
			}
			None => auto("log shard", pool_bytes(&log.space, log.used_bytes))?,
		};
		let cache_bytes = match cache.configured_bytes {
			Some(bytes) => {
				check_fits(
					"disk cache",
					bytes,
					usable_bytes(&cache.space, cache.used_bytes),
				)?;
				bytes
			}
			None => auto("disk cache", pool_bytes(&cache.space, cache.used_bytes))?,
		};
		return Ok((log_bytes, Some(cache_bytes)));
	}

	// One filesystem: both directories draw from a single pool.
	let used = log.used_bytes.saturating_add(cache.used_bytes);
	let usable = usable_bytes(&log.space, used);
	let pool = pool_bytes(&log.space, used);
	match (log.configured_bytes, cache.configured_bytes) {
		(Some(log_bytes), Some(cache_bytes)) => {
			check_fits(
				"log shard and disk cache together",
				log_bytes.saturating_add(cache_bytes),
				usable,
			)?;
			Ok((log_bytes, Some(cache_bytes)))
		}
		(Some(log_bytes), None) => {
			check_fits("log shard", log_bytes, usable)?;
			let cache_bytes = auto("disk cache", pool.saturating_sub(log_bytes))?;
			Ok((log_bytes, Some(cache_bytes)))
		}
		(None, Some(cache_bytes)) => {
			check_fits("disk cache", cache_bytes, usable)?;
			let log_bytes = auto("log shard", pool.saturating_sub(cache_bytes))?;
			Ok((log_bytes, Some(cache_bytes)))
		}
		(None, None) => {
			let pool = auto("log shard and disk cache", pool)?;
			let log_bytes = percent_of(pool, AUTO_LOG_SHARE_PERCENT);
			Ok((log_bytes, Some(pool - log_bytes)))
		}
	}
}

fn percent_of(bytes: u64, percent: u64) -> u64 {
	(u128::from(bytes) * u128::from(percent) / 100) as u64
}

fn check_fits(what: &str, configured: u64, usable: u64) -> Result<()> {
	if configured > usable {
		bail!(
			"configured {what} capacity of {configured} bytes exceeds the {usable} bytes free on its filesystem; lower storage.log_shard.max_disk_bytes or storage.cold_store.disk_cache_bytes, unset them to size from free space, or free disk"
		);
	}
	Ok(())
}

fn auto(what: &str, bytes: u64) -> Result<u64> {
	ensure!(
		bytes > 0,
		"no free disk is left for the {what} after keeping a reserve of {RESERVE_PERCENT}% or {MIN_RESERVE_BYTES} bytes; free disk or configure storage.log_shard.max_disk_bytes and storage.cold_store.disk_cache_bytes"
	);
	Ok(bytes)
}

/// Reads the filesystems of the log directory and cache directory, creating the cache
/// directory, and resolves both capacities. Logs the result and publishes it as metrics.
#[cfg(not(target_arch = "wasm32"))]
pub async fn resolve(
	fs: &Fs,
	depot: &rivet_config::config::depot::DepotConfig,
) -> Result<Option<DiskBudget>> {
	let Some(log_directory) = depot.log_directory.as_deref() else {
		return Ok(None);
	};
	let log = placement(fs, log_directory, depot.log_shard.max_disk_bytes, None).await?;

	let cold_store = &depot.cold_store;
	let cache_enabled = cold_store.disk_cache_bytes != Some(0);
	let cache_directory = cold_store
		.disk_cache_directory
		.clone()
		.unwrap_or_else(|| log_directory.join(DEFAULT_CACHE_DIRECTORY));
	let cache = if cache_enabled {
		fs.create_dir_all(&cache_directory).await.with_context(|| {
			format!(
				"failed to create disk cache directory {}",
				cache_directory.display()
			)
		})?;
		// Cache files inside the log directory are already counted in the log's usage.
		let nested = cache_directory.starts_with(log_directory);
		Some(
			placement(
				fs,
				&cache_directory,
				cold_store.disk_cache_bytes,
				Some(nested),
			)
			.await?,
		)
	} else {
		None
	};

	let (log_disk_bytes, cache_bytes) = plan(log, cache)?;
	let part_bytes = usize::try_from(cold_store.disk_cache_part_bytes())
		.context("storage.cold_store.disk_cache_part_bytes does not fit in memory")?;
	ensure!(
		part_bytes > 0 && part_bytes % 1024 == 0,
		"storage.cold_store.disk_cache_part_bytes must be a positive multiple of 1024"
	);
	let budget = DiskBudget {
		log_disk_bytes,
		cache: cache_bytes.map(|capacity_bytes| DiskCacheBudget {
			directory: cache_directory,
			capacity_bytes,
			part_bytes,
		}),
	};
	tracing::info!(
		log_directory = %log_directory.display(),
		log_disk_bytes,
		log_configured = depot.log_shard.max_disk_bytes.is_some(),
		cache_directory = budget.cache.as_ref().map(|cache| cache.directory.display().to_string()),
		cache_bytes,
		cache_configured = cold_store.disk_cache_bytes.is_some(),
		"resolved storage disk capacities"
	);
	crate::metrics::publish_disk_budget(&budget);
	Ok(Some(budget))
}

/// The metrics recorder for the disk cache this budget sizes. It exports
/// `rivet_depot_cold_store_disk_cache_parts_total{result="access"|"hit"}`, the cache's
/// usage and its evictions.
#[cfg(not(target_arch = "wasm32"))]
pub fn cache_metrics_recorder() -> std::sync::Arc<dyn slatedb_common::metrics::MetricsRecorder> {
	crate::metrics::disk_cache_recorder()
}

#[cfg(not(target_arch = "wasm32"))]
async fn placement(
	fs: &Fs,
	directory: &Path,
	configured_bytes: Option<u64>,
	counted_by_log: Option<bool>,
) -> Result<Placement> {
	let space = fs
		.disk_space(directory)
		.await
		.with_context(|| format!("failed to read free disk space for {}", directory.display()))?;
	let used_bytes = if counted_by_log == Some(true) {
		0
	} else {
		fs.directory_bytes(directory)
			.await
			.with_context(|| format!("failed to measure {}", directory.display()))?
	};
	Ok(Placement {
		space,
		used_bytes,
		configured_bytes,
	})
}


