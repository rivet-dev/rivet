//! Selects the storage worker's identity before topology and UPS are constructed.

use anyhow::{Context, Result, ensure};
use depot_log::log_shard_machine::{
	executor::LogShardExecutor,
	state::LogShardState,
	types::{LOADED_REPLICA_BYTES, LogShardConfig},
};
use depot_types::{
	keys::fs::{LogShardFile, LogShardKey},
	log_shard::read_format,
};
use rivet_env::{Env, fs::File};
use rivet_keys::keys::codec::KeyAddress;
use rivet_topology::{heartbeat::DirectoryLock, worker::WorkerIdentity};
use std::{io::ErrorKind, path::Path, sync::Arc};

/// Retained by process bootstrap until the shard and its filesystem tasks have joined.
/// Dropping it releases the directory lock, including after an abrupt process exit.
///
/// The lock is also death evidence for the next process of this worker, and topology
/// advertises `lock` in every heartbeat. A later process that takes the lock under the same
/// scope knows the process that advertised it has released it, and claims its heartbeat
/// without waiting out the lease. That is only sound if releasing the lock means the process
/// can no longer touch storage, so the engine keeps this directory until its runtime has
/// dropped every task, and the lock ends only with the process. Never release it earlier
/// while the process keeps running, for example to reopen the directory in the same process.
pub struct StorageDirectory {
	pub identity: WorkerIdentity,
	/// The scope of the lock this directory holds, or `None` when the filesystem cannot name
	/// one, in which case a restart waits out the previous process's lease.
	pub lock: Option<DirectoryLock>,
	/// The shard limits the directory was opened with, including its resolved disk capacity.
	pub limits: LogShardConfig,
	_directory: File,
}

impl StorageDirectory {
	/// Locks an existing directory and initializes only an empty shard. An existing FORMAT
	/// restores the shared worker ID; the process incarnation is always newly generated.
	/// This does not start recovery or any background service.
	#[tracing::instrument(skip_all)]
	pub async fn open(env: Arc<dyn Env>, root: &Path, config: &LogShardConfig) -> Result<Self> {
		// Validate before touching disk, then exclude another process for the entire lifetime.
		LogShardState::validate_config(config)?;
		let fs = env.fs();
		fs.read_dir(root)
			.await
			.context("Depot log_directory must already exist")?;
		let directory = fs.open(root).await?;
		directory
			.try_lock()
			.await
			.context("Depot log_directory is already in use or cannot be locked")?;
		let lock = match directory.lock_scope().await {
			Ok(scope) => Some(DirectoryLock {
				kernel_boot_id: scope.kernel_boot_id,
				device: scope.device,
				inode: scope.inode,
			}),
			Err(error) => {
				tracing::warn!(
					?error,
					"log directory lock has no scope; a restart waits out the previous process's lease"
				);
				None
			}
		};

		// Never replace malformed or foreign-version storage with a fresh identity.
		let mut identity = WorkerIdentity::new(&mut env.rng("depot.identity"));
		let format_path = root.join(LogShardKey::new(LogShardFile::Format).pack());
		match fs.read(&format_path).await {
			Ok(bytes) => {
				identity.worker_id = read_format(&bytes, &config.read_versions)?.worker_id;
				fs.open(&format_path).await?.sync_all().await?;
			}
			Err(error) if error.kind() == ErrorKind::NotFound => {
				LogShardExecutor::create(
					fs,
					root.to_owned(),
					&identity.worker_id,
					env.rng("depot.disk-incarnation").uuid_v4(),
					config.max_batch_bytes as usize,
					config.segment_bytes,
				)
				.await?;
			}
			Err(error) => return Err(error.into()),
		}

		tracing::info!(worker_id = %identity.worker_id, "selected storage worker identity");
		Ok(Self {
			identity,
			lock,
			limits: config.clone(),
			_directory: directory,
		})
	}
}

/// The shard's replica cap: its byte budget at the shard's per-replica charge, capped by the
/// optional count.
fn loaded_replica_cap(
	config: &rivet_config::config::depot::LogShardConfig,
	default: u32,
) -> Result<u32> {
	let by_bytes = match config.max_loaded_volume_bytes {
		Some(bytes) => {
			let replicas = bytes / LOADED_REPLICA_BYTES;
			u32::try_from(replicas).unwrap_or(u32::MAX)
		}
		None => default,
	};
	let cap = match config.max_loaded_volumes {
		Some(count) => by_bytes.min(count),
		None => by_bytes,
	};
	ensure!(
		cap > 0,
		"storage.log_shard.max_loaded_volume_bytes and max_loaded_volumes must allow at least one replica"
	);
	Ok(cap)
}

/// Resolves configuration at the service boundary; the shard remains the owner of defaults.
pub fn log_shard_config(
	config: &rivet_config::config::depot::LogShardConfig,
) -> Result<LogShardConfig> {
	let defaults = LogShardConfig::default();
	let max_memory_bytes = config.max_memory_bytes.unwrap_or(defaults.max_memory_bytes);
	let max_admission_wait_ms = config
		.max_admission_wait_ms
		.unwrap_or(defaults.max_admission_wait_ms);
	let resolved = LogShardConfig {
		max_memory_requests: config
			.max_memory_requests
			.unwrap_or(defaults.max_memory_requests),
		inbox_capacity: config.inbox_capacity.unwrap_or(defaults.inbox_capacity),
		max_memory_bytes,
		max_batch_bytes: config.max_batch_bytes.unwrap_or(defaults.max_batch_bytes),
		max_write_bytes: config.max_write_bytes.unwrap_or(defaults.max_write_bytes),
		max_disk_bytes: config.max_disk_bytes.unwrap_or(defaults.max_disk_bytes),
		write_to_cold_at_disk_percent: config
			.write_to_cold_at_disk_percent
			.unwrap_or(defaults.write_to_cold_at_disk_percent),
		max_loaded_volumes: loaded_replica_cap(config, defaults.max_loaded_volumes)?,
		write_to_disk_timeout_ms: config
			.write_to_disk_timeout_ms
			.unwrap_or(defaults.write_to_disk_timeout_ms),
		recovery_timeout_ms: config
			.recovery_timeout_ms
			.unwrap_or(defaults.recovery_timeout_ms),
		manifest_refresh_timeout_ms: config
			.manifest_refresh_timeout_ms
			.unwrap_or(defaults.manifest_refresh_timeout_ms),
		checkpoint_timeout_ms: config
			.checkpoint_timeout_ms
			.unwrap_or(defaults.checkpoint_timeout_ms),
		remove_segments_timeout_ms: config
			.remove_segments_timeout_ms
			.unwrap_or(defaults.remove_segments_timeout_ms),
		segment_bytes: config.segment_bytes.unwrap_or(defaults.segment_bytes),
		preallocate_segments: config.preallocate_segments.unwrap_or(true),
		checkpoint_bytes: config.checkpoint_bytes.unwrap_or(defaults.checkpoint_bytes),
		// The storage service gives the maintainer this same limit for its forget and trim
		// batches.
		max_forget_batch_volumes: defaults.max_forget_batch_volumes,
		max_retained_payload_bytes: config
			.max_retained_payload_bytes
			.unwrap_or(max_memory_bytes / 16),
		max_retained_payload_bytes_per_volume: config
			.max_retained_payload_bytes_per_volume
			.unwrap_or(defaults.max_retained_payload_bytes_per_volume),
		release_idle_payloads_after_ms: config
			.release_idle_payloads_after_ms
			.unwrap_or(defaults.release_idle_payloads_after_ms),
		max_disk_reads: config.max_disk_reads.unwrap_or(defaults.max_disk_reads),
		disk_read_concurrency: config
			.disk_read_concurrency
			.unwrap_or(defaults.disk_read_concurrency),
		append_notice_delay_ms: defaults.append_notice_delay_ms,
		max_notice_entry_bytes: defaults.max_notice_entry_bytes,
		max_notice_request_entry_bytes: defaults.max_notice_request_entry_bytes,
		notice_timeout_ms: defaults.notice_timeout_ms,
		// What this binary advertises in its topology registration.
		read_versions: defaults.read_versions,
		max_admission_wait_ms,
		max_append_admission_wait_ms: config.max_append_admission_wait_ms.unwrap_or(
			defaults
				.max_append_admission_wait_ms
				.max(max_admission_wait_ms),
		),
		overload_retry_after_ms: config
			.overload_retry_after_ms
			.unwrap_or(defaults.overload_retry_after_ms),
		// Only the system sim models runner time; deployments spend real time.
		simulated_request_cost_us: 0,
		// The storage service reads only resource gauges from its shard snapshot.
		publish_replica_views: false,
	};
	LogShardState::validate_config(&resolved)?;
	Ok(resolved)
}
