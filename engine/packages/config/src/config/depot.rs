use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "StorageConfig")]
pub struct DepotConfig {
	#[serde(default)]
	pub balancing: TabletBalancingConfig,
	/// Bounded, deadline-driven volume alarm delivery.
	#[serde(default)]
	pub alarms: AlarmConfig,
	/// Stop-permit retry interval. Defaults to one second.
	pub stop_poll_ms: Option<u64>,
	/// Maximum coordinated stop wait from the first signal. Defaults to eight minutes.
	pub stop_timeout_ms: Option<u64>,
	/// Teardown budget after a stop permit or timeout. Defaults to one minute.
	pub stop_teardown_ms: Option<u64>,
	#[serde(default)]
	pub checkpoints: CheckpointConfig,
	#[serde(default)]
	pub forks: ForkConfig,
	/// Cold-write age from the first unflushed entry, plus up to a tenth of it as jitter.
	/// Defaults to one minute. Storage workers require the age with its jitter to stay below
	/// `stop_timeout_ms`, so a stopping worker never waits on a slower tiering cadence than
	/// its own stop budget.
	pub write_to_cold_after_ms: Option<u64>,
	pub idle_unload_after_ms: Option<u64>,
	/// How long a partition replica-set change may stay in Learning before the coordinator
	/// aborts it and the partition returns to its old members. Defaults to one hour and must be
	/// at least 45 minutes. It counts from the phase's start, not from the learner's
	/// progress, so it must outlast the membership holder wait (15 minutes) plus the copy of
	/// the largest log suffix a learner may have to catch up.
	pub learning_deadline_ms: Option<u64>,
	/// Committed suffix threshold, default 64 MiB and maximum 256 MiB.
	pub write_to_cold_above_bytes: Option<u64>,
	/// Unflushed applied bytes at which a volume joins a partition flush another volume
	/// requested. Defaults to 16 KiB.
	pub flush_piggyback_min_bytes: Option<u64>,
	/// Throttle for memtable flushes that nobody urgently waits on: a pass that comes due
	/// waits up to this long, so passes that come due close together share one upload and
	/// one L0 SST. Defaults to ten seconds.
	pub memtable_flush_throttle_ms: Option<u64>,
	#[serde(default)]
	pub filesystem: FilesystemConfig,
	#[serde(default)]
	pub kv_commands: KvCommandConfig,
	#[serde(default)]
	pub cold_store: ColdStoreConfig,
	/// Exact replica subject capacity, shared by local clients and the shard. Defaults to 2048.
	pub log_request_capacity: Option<usize>,
	/// Log shard requests a client keeps in flight to one storage worker. Later requests queue
	/// and go out together as one batch when a slot frees. Defaults to 2.
	pub log_client_inflight_per_worker: Option<usize>,
	/// Exact coordinator subject capacity. Defaults to 256.
	pub coordinator_request_capacity: Option<usize>,
	/// Existing directory owned by one storage process. Required when storage services start.
	pub log_directory: Option<std::path::PathBuf>,
	/// Creates this many partitions over this storage worker when the cluster has none, so a
	/// single-process engine serves actors without an operator. A restart finishes a run that
	/// stopped partway; partitions it did not start are left alone. Clusters with several storage
	/// workers leave it unset and run `storage partition provision` instead.
	#[serde(rename = "auto_provision_partitions")]
	pub auto_provision_tablets: Option<usize>,
	/// Optional overrides for the shard's disk, memory and operation bounds.
	#[serde(default)]
	pub log_shard: LogShardConfig,
	/// Fallback interval for re-reading the partition manifest. Routing misses and
	/// `NotMaintainer` replies refresh it at once. Defaults to 30 seconds.
	pub manifest_refresh_ms: Option<u64>,
	/// Per-subject lifecycle request capacity, isolated from other UPS traffic. Defaults to 1024.
	pub volume_request_capacity: Option<usize>,
	/// Entire lifecycle RPC budget, including route refreshes. Defaults to five seconds.
	pub volume_request_timeout_ms: Option<u64>,
	#[serde(default)]
	pub volume_client: VolumeClientConfig,
	/// Background heartbeat reclamation; separate from membership lease expiration.
	#[serde(default)]
	pub heartbeat_gc: HeartbeatGcConfig,
	/// Maximum cached endpoint hints, including missing workers. Defaults to 4096.
	#[serde(default)]
	pub endpoint_cache_capacity: Option<usize>,
	/// Minimum milliseconds between miss-triggered membership refresh attempts, including failures.
	pub membership_refresh_cooldown_ms: Option<u64>,
	/// Concurrent topology endpoint lookups and heartbeat inspections admitted per process. Defaults to 128.
	pub lookup_request_capacity: Option<usize>,
	/// Most events of one durable state machine applied in one commit. Events that arrive while
	/// a commit is in flight are grouped up to this bound. Defaults to 16.
	pub durable_max_events_per_commit: Option<u32>,
	/// Floor for every typed storage command deadline, in milliseconds. Each command keeps its own
	/// default (5 seconds generic, 30 seconds for actor storage); when set, the larger of the two
	/// bounds both the client's retry budget and the maintainer's handler timeout. Unset keeps the
	/// per-command defaults.
	pub command_timeout_ms: Option<u64>,
	/// Client cold-store reads in flight per partition maintainer: session opens, lifecycle row
	/// lookups and session reads share this window. A cold wake's reads fan out through it, so
	/// a narrow window serializes them. Object storage throughput scales almost linearly to
	/// about 32 parallel GETs, and a read slot is not always a GET, so 64 keeps a cold wake's
	/// reads from waiting on slots; on the load cluster a fully cold wake took about 850 ms at
	/// 4, 430 ms at 64 and 390 ms at 512. Defaults to 64.
	pub max_cold_reads: Option<u32>,
	/// Client session reads of one volume a partition maintainer admits while reads of other volumes wait, so one large scan cannot hold every client cold-read slot. Slots no other volume waits for still go to that volume. Unset or 0 admits client reads in arrival order.
	pub max_cold_reads_per_volume: Option<u32>,
	/// Chunk runs of at least this many chunks (4 KiB each) are read with the bulk scan settings below, as a table scan's read-ahead windows are. Unset or 0 reads every run with the normal settings.
	pub bulk_read_min_chunks: Option<usize>,
	/// Whether SST blocks a bulk run fetches enter the shared block cache. Defaults to true; false keeps large scans from evicting other volumes' hot blocks.
	pub bulk_read_cache_blocks: Option<bool>,
	/// Concurrent SST block fetches inside one bulk run's scan. Defaults to 1; 0 is treated as 1.
	pub bulk_read_fetch_tasks: Option<usize>,
	/// Bytes per second that bulk runs may read across every partition of this process, with a burst of a quarter second (at least 4 MiB). Bulk runs wait for their bytes; point reads never wait. Unset or 0 leaves bulk runs unpaced.
	pub bulk_read_bytes_per_sec: Option<u64>,
	/// Size of each background cold-read window per partition maintainer: held-image source reads
	/// for forks, replica log reads for applies, fork jobs and fork cleanup jobs are each bounded
	/// by it separately, so they never take client read slots. Defaults to 4.
	pub max_background_cold_reads: Option<u32>,
	/// Simulation only: processing time each filesystem read request costs a partition maintainer, in
	/// microseconds, spent on the simulated clock so a maintainer busy with reads queues them
	/// as a loaded one does. Unset in deployments, where real processing time passes on its own.
	pub simulated_fs_read_cost_us: Option<u64>,
	/// Filesystem read sessions per partition maintainer. A session lives while its volume is
	/// being read and ends after `read_session_idle_ms` without a read, so this bounds how many
	/// volumes a partition serves reads for at once. Opens past it are refused as overloaded. A
	/// held session retains about 1.2 KiB of maintainer heap, so the default of 16,384 holds under
	/// 20 MiB per partition. Must be positive.
	pub max_read_sessions: Option<usize>,
	/// How long a read session stays registered after its last read, in milliseconds. Each
	/// read extends it up to the session's lifetime, and a client reopens an ended session on
	/// its next read, so sessions track volumes being read rather than volumes open. Must be
	/// positive. Defaults to 2,000.
	pub read_session_idle_ms: Option<u64>,
	/// How long a session opened at the latest position stays registered after its last read,
	/// in milliseconds. An HTTP directory cursor resumes such a session between pages, and no
	/// client reopens it at its position, so a paging client keeps its cursor for this long
	/// between pages. Must be positive. Defaults to 30,000, the session lifetime.
	pub read_cursor_session_idle_ms: Option<u64>,
	/// Codec for SlateDB SST blocks in the cold store; defaults to `lz4`. Each SST records
	/// its own codec, so changing it keeps existing SSTs readable.
	pub cold_store_compression: Option<ColdStoreCompression>,
	/// Volume records whose old rows one partition filesystem flush reads at once while it
	/// stages. Staging excludes every other store operation of the partition. Defaults to 16.
	pub flush_stage_concurrency: Option<usize>,
	/// Filesystem applies and loads one partition maintainer admits to its store at once. They
	/// run concurrently, one task each, and their replica reads stay bounded by the cold
	/// byte budget, so the cap sizes concurrency for a burst of volume creates rather than
	/// memory. Must be positive. Defaults to 256.
	pub store_max_keyed_in_flight: Option<u32>,
	/// Partition filesystem flushes one partition maintainer lets stage at once. Staging writes the
	/// flush's records into the store. Must be positive. Defaults to 1.
	pub store_max_flush_staging: Option<u32>,
	/// Partition filesystem flushes one partition maintainer keeps in flight at once, staging or
	/// uploading; a staging flush counts because it uploads next. Must be at least
	/// `store_max_flush_staging`. Defaults to 2.
	pub store_max_flush_uploads: Option<u32>,
	/// Queued store items one admission pass looks at past work that cannot start yet.
	/// Work beyond it waits for the next pass. Must be positive. Defaults to 1024.
	pub store_admission_scan_limit: Option<u32>,
	/// Volume lock stripes of each partition filesystem store. Two volumes on one stripe
	/// serialize their store work. Must be positive. Defaults to 1024.
	pub store_volume_lock_stripes: Option<usize>,
	/// Which store work takes volume stripes. `core` (the default) trusts the maintainer
	/// core to order applies, loads, session opens, lifecycle lookups and the flush
	/// retention publish, so they take only the partition lock; stripes then guard only
	/// checkpoint-session opens against hold releases, checkpoint deletions and sweeps.
	/// `stripes` keeps a stripe on every operation, as before.
	pub store_volume_locks: Option<StoreVolumeLocks>,
	/// Queued events one partition maintainer accepts before it rejects new requests as
	/// overloaded. Every volume open, read, apply and status notice for the partition passes
	/// through it, so a burst of actor starts on one partition needs room for several events per
	/// start. Must be positive. Defaults to 4096.
	pub maintainer_inbox_capacity: Option<u32>,
	/// Client requests one partition maintainer tracks at once before it rejects more as
	/// overloaded. Must be positive. Defaults to 1024.
	pub maintainer_max_requests: Option<u32>,
	/// Volume opens queued in one partition maintainer's inbox, not yet taken by its processing task, at
	/// which a new open is refused as overloaded at once instead of joining the inbox. Other
	/// inbox events do not count. Opens queued past this would mostly expire before
	/// admission. Must be positive. Defaults to 1024.
	pub maintainer_max_queued_opens: Option<u32>,
	/// Volumes one partition maintainer holds loaded at once, before opens of more are refused as
	/// overloaded. A loaded volume costs about 1.8 KiB of maintainer heap, so the default of
	/// 500,000 holds under 1 GiB per partition. Must be positive.
	pub maintainer_max_loaded_volumes: Option<u32>,
}

/// Floor for `learning_deadline_ms`: the partition maintainer's 15 minute membership holder wait
/// plus 30 minutes for the learner's copy.
const MIN_LEARNING_DEADLINE_MS: u64 = (15 + 30) * 60 * 1000;

impl DepotConfig {
	pub fn stop_poll_ms(&self) -> u64 {
		self.stop_poll_ms.unwrap_or(1000)
	}
	pub fn stop_timeout_ms(&self) -> u64 {
		self.stop_timeout_ms.unwrap_or(8 * 60 * 1000)
	}
	pub fn stop_teardown_ms(&self) -> u64 {
		self.stop_teardown_ms.unwrap_or(60 * 1000)
	}
	pub fn learning_deadline_ms(&self) -> u64 {
		self.learning_deadline_ms.unwrap_or(60 * 60 * 1000)
	}
	pub fn write_to_cold_after_ms(&self) -> u64 {
		self.write_to_cold_after_ms.unwrap_or(60 * 1000)
	}
	/// The longest tiering age a volume can wait: the configured age plus its jitter of up to a
	/// tenth, as the partition maintainer schedules it.
	pub fn max_write_to_cold_age_ms(&self) -> u64 {
		let age = self.write_to_cold_after_ms();
		age.saturating_add(age / 10)
	}
	/// Only storage processes need the permit wait; service selection owns this validation.
	pub fn validate_storage_shutdown(&self, force: std::time::Duration) -> Result<()> {
		self.validate()?;
		if self.stop_poll_ms() == 0 || self.stop_timeout_ms() == 0 || self.stop_teardown_ms() == 0 {
			bail!("Depot stop durations must be positive");
		}
		let Some(total) = self.stop_timeout_ms().checked_add(self.stop_teardown_ms()) else {
			bail!("Depot stop durations overflow");
		};
		if force <= std::time::Duration::from_millis(total) {
			bail!(
				"runtime.force_shutdown_duration must exceed storage.stop_timeout_ms + storage.stop_teardown_ms for storage workers"
			);
		}
		if self.max_write_to_cold_age_ms() >= self.stop_timeout_ms() {
			bail!(
				"storage.write_to_cold_after_ms plus its 10% jitter ({} ms) must stay below storage.stop_timeout_ms ({} ms) for storage workers",
				self.max_write_to_cold_age_ms(),
				self.stop_timeout_ms()
			);
		}
		Ok(())
	}

	pub fn log_request_capacity(&self) -> usize {
		self.log_request_capacity.unwrap_or(2048)
	}

	pub fn log_client_inflight_per_worker(&self) -> usize {
		self.log_client_inflight_per_worker.unwrap_or(2)
	}

	pub fn coordinator_request_capacity(&self) -> usize {
		self.coordinator_request_capacity.unwrap_or(256)
	}

	pub fn manifest_refresh_ms(&self) -> u64 {
		self.manifest_refresh_ms.unwrap_or(30_000)
	}

	pub fn volume_request_capacity(&self) -> usize {
		self.volume_request_capacity.unwrap_or(1024)
	}

	pub fn volume_request_timeout_ms(&self) -> u64 {
		self.volume_request_timeout_ms.unwrap_or(5000)
	}

	pub fn endpoint_cache_capacity(&self) -> usize {
		self.endpoint_cache_capacity.unwrap_or(4096)
	}

	pub fn membership_refresh_cooldown(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.membership_refresh_cooldown_ms.unwrap_or(1_000))
	}

	pub fn lookup_request_capacity(&self) -> usize {
		self.lookup_request_capacity.unwrap_or(128)
	}

	pub fn durable_max_events_per_commit(&self) -> u32 {
		self.durable_max_events_per_commit.unwrap_or(16)
	}

	/// `None` keeps each command's own deadline; see `command_timeout_ms`.
	pub fn command_timeout_floor(&self) -> Option<std::time::Duration> {
		self.command_timeout_ms
			.map(std::time::Duration::from_millis)
	}

	pub fn validate(&self) -> Result<()> {
		self.balancing.validate()?;
		if self.auto_provision_tablets == Some(0) {
			bail!("storage.auto_provision_partitions must be positive when set");
		}
		if self.learning_deadline_ms() < MIN_LEARNING_DEADLINE_MS {
			bail!(
				"storage.learning_deadline_ms must be at least {MIN_LEARNING_DEADLINE_MS}: the partition maintainer waits 15 minutes for a held writer to match a learner, and a shorter Learning phase aborts replacements that are still making progress"
			);
		}
		if self.stop_poll_ms() == 0
			|| self.stop_timeout_ms() == 0
			|| self.stop_teardown_ms() == 0
			|| self.log_request_capacity() == 0
			|| self.log_client_inflight_per_worker() == 0
			|| self.coordinator_request_capacity() == 0
			|| self.manifest_refresh_ms() == 0
			|| self.volume_request_capacity() == 0
			|| self.volume_request_timeout_ms() == 0
		{
			bail!("Depot manifest and volume transport bounds must be positive");
		}
		self.volume_client.validate()?;
		self.heartbeat_gc.validate()?;
		self.filesystem.validate()?;
		self.checkpoints.validate()?;
		self.forks.validate()?;
		if self.idle_unload_after_ms == Some(0)
			|| self.write_to_cold_after_ms == Some(0)
			|| self
				.write_to_cold_above_bytes
				.is_some_and(|bytes| !(1..=256 * 1024 * 1024).contains(&bytes))
		{
			bail!("cold-write age must be positive and suffix threshold must be 1..=256 MiB");
		}
		if [
			self.kv_commands.request_capacity,
			self.kv_commands.max_open_volumes,
			self.kv_commands.max_worker_open_volumes,
			self.kv_commands.max_inflight_commands,
			self.kv_commands.max_command_bytes,
			self.kv_commands.max_inflight_command_bytes,
			self.kv_commands.max_eviction_closes,
		]
		.contains(&Some(0))
			|| self.kv_commands.max_worker_open_volume_bytes == Some(0)
			|| self.kv_commands.volume_idle_close_ms == Some(0)
			|| self.kv_commands.short_idle_close_ms == Some(0)
		{
			bail!("Depot KV command limits must be positive");
		}
		if self.endpoint_cache_capacity() == 0
			|| i64::try_from(self.endpoint_cache_capacity()).is_err()
		{
			bail!("storage.endpoint_cache_capacity must be between 1 and i64::MAX");
		}

		if self.membership_refresh_cooldown().is_zero() {
			bail!("storage.membership_refresh_cooldown_ms must be positive");
		}
		if self.lookup_request_capacity() == 0
			|| i64::try_from(self.lookup_request_capacity()).is_err()
		{
			bail!("storage.lookup_request_capacity must be between 1 and i64::MAX");
		}
		if !(1..=1_024).contains(&self.durable_max_events_per_commit()) {
			bail!("storage.durable_max_events_per_commit must be between 1 and 1024");
		}
		if self.command_timeout_ms == Some(0) {
			bail!("storage.command_timeout_ms must be positive");
		}
		if self.read_session_idle_ms == Some(0) || self.read_cursor_session_idle_ms == Some(0) {
			bail!(
				"storage.read_session_idle_ms and storage.read_cursor_session_idle_ms must be positive"
			);
		}
		if self.max_read_sessions == Some(0) {
			bail!("storage.max_read_sessions must be positive");
		}
		if self.cold_store.unflushed_budget_bytes == Some(0)
			|| self.cold_store.max_concurrent_compactions == Some(0)
		{
			bail!(
				"storage.cold_store.unflushed_budget_bytes and max_concurrent_compactions must be positive"
			);
		}
		if self.cold_store.l0_max_ssts.is_some_and(|ssts| ssts < 4) {
			bail!("storage.cold_store.l0_max_ssts must be at least 4");
		}
		if self.max_cold_reads == Some(0) {
			bail!("storage.max_cold_reads must be positive");
		}
		if self.max_background_cold_reads == Some(0) {
			bail!("storage.max_background_cold_reads must be positive");
		}
		Ok(())
	}
}

/// Static partition placement policy; membership and lease timings remain topology-owned.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(rename = "PartitionBalancingConfig")]
pub struct TabletBalancingConfig {
	/// Whether the partition coordinator places, splits and replaces partitions on its own. When
	/// disabled, the layout changes only through operator commands: a new cluster gets no
	/// first partition until an operator creates one, and a missing worker is never replaced.
	pub enabled: bool,
	#[serde(rename = "partitions_per_core")]
	pub tablets_per_core: u32,
	/// How long a new worker must be listed in discovery, and pass one disk probe, before the
	/// coordinator places partitions on it. Must exceed the topology worker lease plus one renewal
	/// and the takeover grace, about 35 s, so a worker that crashes soon after it first appears has
	/// left discovery before it qualifies. Startup refuses a shorter value.
	pub destination_stability_ms: u64,
	/// How long an assigned worker must be missing from discovery before its replicas are
	/// replaced. Counted from when discovery drops it, which is itself one lease after its
	/// last renewal.
	pub missing_worker_ms: u64,
	/// Maximum concurrent disk admission requests made by the partition coordinator.
	pub max_disk_probes: u32,
}
impl Default for TabletBalancingConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			tablets_per_core: 2,
			destination_stability_ms: 60_000,
			missing_worker_ms: 120_000,
			max_disk_probes: 4,
		}
	}
}
impl TabletBalancingConfig {
	/// Unit conversion for the CPU capacity advertised by storage workers.
	pub const MILLICORES_PER_CORE: u32 = 1000;

	pub fn validate(&self) -> Result<()> {
		if self.tablets_per_core == 0
			|| self.destination_stability_ms == 0
			|| self.missing_worker_ms == 0
			|| self.max_disk_probes == 0
		{
			bail!(
				"Depot balancing weights, observation durations and probe limit must be positive"
			);
		}
		Ok(())
	}
}

/// Which partition store operations take volume lock stripes.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoreVolumeLocks {
	/// Only work the maintainer core does not order takes a stripe.
	Core,
	/// Every store operation takes its volumes' stripes.
	Stripes,
}

/// Block compression for cold-store SSTs.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColdStoreCompression {
	None,
	Lz4,
}

/// Optional overrides for topology's bounded heartbeat cleanup service, which runs under the
/// storage owner's singleton election. Topology defines the resolved defaults.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HeartbeatGcConfig {
	/// Defaults to 24 hours; clean shutdown markers remain available during this window.
	pub retention_ms: Option<u64>,
	/// LIST entries examined per batch, 1 to 1000. Defaults to 128.
	pub batch_size: Option<usize>,
	/// Conditional deletes in flight per batch, 1 to `batch_size`. Defaults to 4.
	pub delete_concurrency: Option<usize>,
	/// Time budget of one batch. Defaults to 2 seconds.
	pub batch_timeout_ms: Option<u64>,
	/// Pause between the batches of one pass. Defaults to 10 seconds.
	pub batch_interval_ms: Option<u64>,
	/// Time between the starts of consecutive passes over both heartbeat prefixes. Defaults
	/// to one hour.
	pub pass_interval_ms: Option<u64>,
}

impl HeartbeatGcConfig {
	/// Rejects zero and out-of-range bounds. Retention against the lease timing, and the
	/// concurrency against a defaulted batch size, are checked where the collector starts,
	/// because topology owns those defaults.
	fn validate(&self) -> Result<()> {
		if [
			self.retention_ms,
			self.batch_timeout_ms,
			self.batch_interval_ms,
			self.pass_interval_ms,
		]
		.contains(&Some(0))
			|| self.batch_size == Some(0)
			|| self.delete_concurrency == Some(0)
		{
			bail!("Depot heartbeat GC bounds must be positive");
		}
		if self.batch_size.is_some_and(|size| size > 1000) {
			bail!("storage.heartbeat_gc.batch_size must be 1..=1000");
		}
		if let (Some(concurrency), Some(size)) = (self.delete_concurrency, self.batch_size) {
			if concurrency > size {
				bail!("storage.heartbeat_gc.delete_concurrency must not exceed batch_size");
			}
		}

		Ok(())
	}
}

/// Optional process ownership overrides; the volume client defines the resolved defaults.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VolumeClientConfig {
	/// Volumes one engine process holds open at once; defaults to 32768. A held volume costs
	/// about 14 KiB of client state and under 64 KiB with its filesystem owner, so the default
	/// stays under 2 GiB. Page caches are bounded separately by the process cache budget.
	pub max_loaded_volumes: Option<usize>,
	pub max_requests: Option<usize>,
	pub max_volume_key_bytes: Option<usize>,
	pub sweep_interval_ms: Option<u64>,
	pub close_after_idle_ms: Option<u64>,
	/// Time an open waits for volume recovery before failing; defaults to 3000 ms.
	pub open_timeout_ms: Option<u64>,
	/// Codec for logged write payloads: `lz4` (the default) or `none`. Each log entry records
	/// its codec, so changing this affects only new writes.
	pub payload_codec: Option<VolumePayloadCodec>,
}

/// Codec for a volume's logged write payloads.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VolumePayloadCodec {
	None,
	Lz4,
}

impl VolumeClientConfig {
	fn validate(&self) -> Result<()> {
		if [
			self.max_loaded_volumes,
			self.max_requests,
			self.max_volume_key_bytes,
		]
		.contains(&Some(0))
			|| [
				self.sweep_interval_ms,
				self.close_after_idle_ms,
				self.open_timeout_ms,
			]
			.contains(&Some(0))
		{
			bail!("Depot volume client bounds must be positive");
		}
		Ok(())
	}
}

/// The log shard owns resolved defaults and validates relationships between these limits.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogShardConfig {
	pub max_memory_requests: Option<u32>,
	pub inbox_capacity: Option<usize>,
	pub max_memory_bytes: Option<u64>,
	pub max_batch_bytes: Option<u32>,
	pub max_write_bytes: Option<u64>,
	/// Disk bytes the shard's log may hold. When unset, a storage worker sizes it from the
	/// free space of its filesystem at startup, shared with the cold-store disk cache.
	pub max_disk_bytes: Option<u64>,
	/// Early cold-write watermark, 1..99 percent of the disk cap; defaults to 75.
	pub write_to_cold_at_disk_percent: Option<u8>,
	/// Memory, in bytes, that one log shard's loaded replicas may hold, charged at 1.5 KiB per
	/// replica, the measured heap of a quiet loaded replica with headroom. A load past it is
	/// refused as `too_many_loaded_volumes`, a typed refusal its open retries. The default of
	/// 1.5 GiB holds 1,048,576 replicas (default 1.5 GiB).
	pub max_loaded_volume_bytes: Option<u64>,
	/// A count backstop to `max_loaded_volume_bytes`; unset by default, so the byte budget alone
	/// sets the shard's replica cap.
	pub max_loaded_volumes: Option<u32>,
	pub write_to_disk_timeout_ms: Option<u64>,
	/// How long log shard recovery may go without finishing another file before the shard
	/// fails (default 120 s). It does not bound the whole scan, which grows with the log.
	pub recovery_timeout_ms: Option<u64>,
	pub manifest_refresh_timeout_ms: Option<u64>,
	pub checkpoint_timeout_ms: Option<u64>,
	pub remove_segments_timeout_ms: Option<u64>,
	pub segment_bytes: Option<u64>,
	/// Writes log segments into files zero-filled to `segment_bytes` ahead of use, so each
	/// commit overwrites allocated blocks and syncs only its data instead of journaling a
	/// file size change. On the load cluster's disks this halves the sync and commit latency.
	/// The unused allocation of the active segment and one spare segment count toward
	/// `max_disk_bytes`. Each write also records, in a header slot synced with it, how far
	/// earlier syncs reached, so a disk that returns synced pages as zeros is reported as
	/// corruption rather than read as free space. Needs `segment_bytes` above 8 KiB.
	/// Defaults to true.
	pub preallocate_segments: Option<bool>,
	pub checkpoint_bytes: Option<u64>,
	/// Encoded bytes of synced log entry payloads the shard keeps in memory for reads;
	/// defaults to a sixteenth of `max_memory_bytes`. Older payloads are read from disk.
	pub max_retained_payload_bytes: Option<u64>,
	/// Synced payload bytes one volume replica keeps after each disk write; defaults to
	/// 32 KiB.
	pub max_retained_payload_bytes_per_volume: Option<u64>,
	/// How long a volume replica appends nothing before its synced payloads leave memory;
	/// defaults to 5000.
	pub release_idle_payloads_after_ms: Option<u64>,
	/// Most reads waiting on the shard's disk at once; a read that needs disk beyond it is
	/// rejected as overloaded and retried by the caller. Defaults to 256.
	pub max_disk_reads: Option<u32>,
	/// Disk reads executing at once beside the shard's write queue. Each holds one scan
	/// buffer of two maximum batches from `max_memory_bytes`. Defaults to 8.
	pub disk_read_concurrency: Option<u32>,
	/// Longest wait, in milliseconds, of a request other than an append in the shard's
	/// inbox. A request that waited longer is refused as overloaded before any work, with a
	/// retry hint, so a saturated shard sheds load instead of queueing past the volume
	/// client's quorum timeout. Defaults to 1000.
	pub max_admission_wait_ms: Option<u64>,
	/// The same bound for appends, commit announcements, trims and forgets, which carry
	/// writes a volume client already admitted or free memory and disk, so opens and other
	/// new work are shed first. At least `max_admission_wait_ms` and just under the volume
	/// client's 2 s append timeout, past which the client has resent the append anyway.
	/// Defaults to 1800.
	pub max_append_admission_wait_ms: Option<u64>,
	/// Retry hint, in milliseconds, on a request refused because the inbox or a request
	/// budget is full. Defaults to 100.
	pub overload_retry_after_ms: Option<u64>,
}

/// SlateDB caches shared by every cold-store database of one engine process.
///
/// These are fixed RAM caps of the storage worker process, separate from the log shard's
/// and filesystems' budgets. With the defaults they hold up to 2 GiB together: 512 MiB of
/// data blocks, 512 MiB of SST metadata and 1 GiB of unflushed memtables. Size a host for
/// them beside `filesystem.max_process_cache_bytes`, the log shard's `max_memory_bytes` and
/// loaded replicas, and the KV volume budget `kv_commands.max_worker_open_volume_bytes`.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColdStoreConfig {
	/// Data block cache bytes for the whole process, not per database. Defaults to 512 MiB.
	pub block_cache_bytes: Option<u64>,
	/// Index, filter and statistics cache bytes for the whole process. Defaults to 512 MiB.
	///
	/// Every SST probe needs that SST's index and filter in this cache, and a miss loads and
	/// decodes the SST's whole index. Indexes are about 2% of SST bytes, so 512 MiB keeps the
	/// metadata of about 25 GiB of SSTs per process in memory. Past that, reads start
	/// evicting indexes other reads need, and each actor start pays for more index decodes.
	/// A smaller cache reaches that point at proportionally less data.
	pub meta_cache_bytes: Option<u64>,
	/// Unflushed memtable bytes for the whole process, split evenly across the partitions this
	/// worker maintains (at most 512 MiB and at least 8 MiB per partition). Defaults to 1 GiB.
	pub unflushed_budget_bytes: Option<u64>,
	/// Concurrent compactions per partition database. A worker maintaining more than 8 partitions
	/// runs one per database. Defaults to 2.
	pub max_concurrent_compactions: Option<usize>,
	/// L0 SSTs a partition database holds before SlateDB stalls memtable flushes until a
	/// compaction drains them. One compaction takes up to this many sources, so it drains a
	/// full L0 at once. Higher values absorb flush bursts at the cost of more SSTs per read.
	/// Defaults to 16.
	pub l0_max_ssts: Option<usize>,
	/// Largest SST a compaction writes before it starts the next one. A read whose index is
	/// not cached loads and decodes the SST's whole index, about 2% of the SST's bytes, so
	/// smaller compacted SSTs keep that cost small once the indexes of a growing database
	/// outgrow `meta_cache_bytes`. Defaults to 16 MiB.
	pub compacted_sst_bytes: Option<u64>,
	/// Directory for the local disk cache of cold-store object parts. Defaults to
	/// `cold-cache` inside `storage.log_directory`. Its contents are disposable.
	pub disk_cache_directory: Option<std::path::PathBuf>,
	/// Disk cache capacity in bytes; 0 disables the cache. When unset, a storage worker
	/// sizes it from the free space of its filesystem at startup, next to the log shard.
	pub disk_cache_bytes: Option<u64>,
	/// Bytes fetched per cache miss and stored per cache file, a multiple of 1 KiB.
	/// Defaults to 1 MiB.
	pub disk_cache_part_bytes: Option<u64>,
	/// Writes each SST a partition flush uploads into the disk cache as well, so its first reads
	/// are local. Defaults to true.
	pub disk_cache_on_flush: Option<bool>,
	/// Writes each SST a compaction uploads into the disk cache as well. Compaction output of
	/// large databases is mostly cold, and caching it costs local disk writes on the disk the
	/// log shard shares, so write-heavy deployments turn it off. Defaults to true.
	pub disk_cache_on_compaction: Option<bool>,
}

impl ColdStoreConfig {
	pub fn block_cache_bytes(&self) -> u64 {
		self.block_cache_bytes.unwrap_or(512 * 1024 * 1024)
	}

	pub fn meta_cache_bytes(&self) -> u64 {
		self.meta_cache_bytes.unwrap_or(512 * 1024 * 1024)
	}

	pub fn unflushed_budget_bytes(&self) -> usize {
		self.unflushed_budget_bytes
			.map_or(1024 * 1024 * 1024, |bytes| {
				bytes.min(usize::MAX as u64) as usize
			})
	}

	pub fn max_concurrent_compactions(&self) -> usize {
		self.max_concurrent_compactions.unwrap_or(2)
	}

	pub fn l0_max_ssts(&self) -> usize {
		self.l0_max_ssts.unwrap_or(16)
	}

	pub fn compacted_sst_bytes(&self) -> usize {
		self.compacted_sst_bytes.map_or(16 * 1024 * 1024, |bytes| {
			bytes.min(usize::MAX as u64) as usize
		})
	}

	pub fn disk_cache_part_bytes(&self) -> u64 {
		self.disk_cache_part_bytes.unwrap_or(1024 * 1024)
	}

	pub fn disk_cache_on_flush(&self) -> bool {
		self.disk_cache_on_flush.unwrap_or(true)
	}

	pub fn disk_cache_on_compaction(&self) -> bool {
		self.disk_cache_on_compaction.unwrap_or(true)
	}
}

/// Optional limits for typed commands and maintainer-owned filesystems.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KvCommandConfig {
	/// KV commands one worker holds received and unanswered, and that one caller may have
	/// outstanding to one worker, isolated from volume lifecycle requests. A full window refuses
	/// a command at once as overloaded, and the caller retries it within its deadline. Each
	/// partition's host admits at most the sum of its commands' concurrency caps (4,608 with the
	/// default caps and the engine's commands), so the default of 32,768 lets those caps, not this
	/// window, refuse load first on a worker that maintains up to seven partitions. Defaults to 32,768.
	pub request_capacity: Option<usize>,
	/// KV command volumes one partition's host holds open at once, closing ones included. Defaults
	/// to the worker-wide cap, so by default only the worker's memory budget limits a partition.
	pub max_open_volumes: Option<usize>,
	/// Memory, in bytes, that the KV command volumes of this worker may hold together across
	/// every partition it maintains, closing ones included. Each open volume is charged 80 KiB, its
	/// measured fixed cost with allocator headroom: its host slot, volume client and filesystem
	/// owner, plus the log replicas and maintainer state its open loads. Caches and command
	/// payloads are bounded separately. One worker-wide budget bounds memory however many
	/// partitions the worker holds, and a create burst opens one volume per new key, so the
	/// default of 5 GiB holds 65,536 volumes (default 5 GiB).
	pub max_worker_open_volume_bytes: Option<u64>,
	/// A count backstop to `max_worker_open_volume_bytes`; unset by default, so the byte budget
	/// alone sets the worker's cap.
	pub max_worker_open_volumes: Option<usize>,
	/// Maximum simultaneous handlers per registered command class.
	pub max_inflight_commands: Option<usize>,
	pub max_command_bytes: Option<usize>,
	/// Received bytes of the KV commands one partition's host has admitted and not finished. A
	/// command that would exceed it is refused at once as unavailable, which callers retry
	/// within their deadline. Must hold at least one command of `max_command_bytes`. Defaults to
	/// 64 MiB, so a worker maintaining four partitions holds at most 256 MiB of command payloads.
	pub max_inflight_command_bytes: Option<usize>,
	/// Defaults to 60 seconds.
	pub volume_idle_close_ms: Option<u64>,
	/// Warns on eviction below this idle time; defaults to 15 seconds.
	pub evicted_idle_warn_ms: Option<u64>,
	/// Idle KV command volumes one partition's host closes at once to make room past its cap or
	/// the worker's budget. Each open refused at a full cap starts one such close, so a burst
	/// admits at this many closes per close latency. A closing volume keeps its memory charge
	/// until its close finishes. Defaults to 32.
	pub max_eviction_closes: Option<usize>,
	/// Idle close for volumes of short-lived commands, such as one Epoxy key per volume.
	/// Closing also begins the volume's unload. Defaults to 2 seconds.
	pub short_idle_close_ms: Option<u64>,
}

impl KvCommandConfig {
	pub fn request_capacity(&self) -> usize {
		self.request_capacity.unwrap_or(32_768)
	}

	pub fn max_worker_open_volume_bytes(&self) -> u64 {
		self.max_worker_open_volume_bytes
			.unwrap_or(5 * 1024 * 1024 * 1024)
	}

	pub fn max_inflight_command_bytes(&self) -> usize {
		self.max_inflight_command_bytes.unwrap_or(64 * 1024 * 1024)
	}

	pub fn max_eviction_closes(&self) -> usize {
		self.max_eviction_closes.unwrap_or(32)
	}

	pub fn short_idle_close(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.short_idle_close_ms.unwrap_or(2_000))
	}
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Eq, PartialEq, Hash)]
#[serde(default, deny_unknown_fields)]
pub struct FilesystemConfig {
	pub max_memory_bytes: usize,
	/// Bytes one filesystem's row cache may retain, a fairness ceiling inside
	/// `max_process_cache_bytes`.
	pub max_cache_bytes: usize,
	pub max_operation_bytes: usize,
	pub max_snapshot_age_ms: u64,
	pub max_snapshots: usize,
	pub max_conflict_window_bytes: usize,
	pub max_value_bytes: usize,
	pub request_timeout_ms: u64,
	pub max_pending_commits: usize,
	/// Maximum application bytes in one filesystem transaction.
	pub max_transaction_bytes: usize,
	pub max_transaction_age_ms: u64,
	/// Bytes all filesystem row caches in one process may retain together.
	pub max_process_cache_bytes: usize,
	/// How long a write's overlay notes ("this page is at this sequence") outlive the write
	/// before the filesystem drops them and keeps their rows only as cache. A later miss on one
	/// of those rows reads it from the maintainer once it has applied the write. About ten
	/// times the usual apply latency.
	pub note_ttl_ms: u64,
	/// Most payload bytes one opening may have admitted ahead of the newest position its
	/// partition's maintainer reported applied. A new write past it is refused with a retry hint
	/// before it is built, so a writer faster than the apply, as under a slow cold store or
	/// compaction, waits at admission instead of growing an unbounded backlog.
	pub max_unapplied_bytes: usize,
}

impl Default for FilesystemConfig {
	fn default() -> Self {
		Self {
			max_memory_bytes: 64 * 1024 * 1024,
			max_cache_bytes: 8 * 1024 * 1024,
			max_operation_bytes: 8 * 1024 * 1024,
			max_snapshot_age_ms: 30_000,
			max_snapshots: 128,
			max_conflict_window_bytes: 4 * 1024 * 1024,
			max_value_bytes: 1024 * 1024,
			request_timeout_ms: 30_000,
			max_pending_commits: 128,
			max_transaction_bytes: 128 * 1024 * 1024,
			max_transaction_age_ms: 300_000,
			max_process_cache_bytes: 1024 * 1024 * 1024,
			note_ttl_ms: 1_000,
			max_unapplied_bytes: 256 * 1024 * 1024,
		}
	}
}

impl FilesystemConfig {
	pub fn validate(&self) -> Result<()> {
		if [
			self.max_memory_bytes,
			self.max_operation_bytes,
			self.max_snapshots,
			self.max_conflict_window_bytes,
			self.max_value_bytes,
			self.max_pending_commits,
			self.max_transaction_bytes,
			self.max_unapplied_bytes,
		]
		.contains(&0)
			|| self.max_transaction_age_ms == 0
			|| self.note_ttl_ms == 0
			|| self.max_snapshot_age_ms == 0
			|| self.request_timeout_ms == 0
		{
			bail!("filesystem memory, operation and snapshot limits must be positive");
		}
		Ok(())
	}
}

/// Global checkpoint defaults; individual volume records can override retention.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckpointConfig {
	#[serde(default = "default_named_checkpoints")]
	pub max_named_checkpoints_per_volume: u32,
	pub retention: Option<RetentionPolicy>,
}
fn default_named_checkpoints() -> u32 {
	64
}
impl Default for CheckpointConfig {
	fn default() -> Self {
		Self {
			max_named_checkpoints_per_volume: 64,
			retention: None,
		}
	}
}
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetentionPolicy {
	pub interval_ms: u64,
	pub count: u32,
}
impl CheckpointConfig {
	pub fn validate(&self) -> Result<()> {
		if !(1..=65536).contains(&self.max_named_checkpoints_per_volume)
			|| self
				.retention
				.as_ref()
				.is_some_and(|p| p.count > 1024 || (p.count > 0 && p.interval_ms == 0))
		{
			bail!("invalid checkpoint count or retention interval");
		}
		Ok(())
	}
}

/// Global bounds for source hold cleanup and background cross-region copies.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ForkConfig {
	/// Minimum age before an unfinished fork's destination is checked.
	pub check_grace_ms: u64,
	pub max_concurrent_materializations: u32,
	/// Further bounded by the maintainer's row and byte budgets.
	pub materialize_batch_chunks: u32,
}
impl Default for ForkConfig {
	fn default() -> Self {
		Self {
			check_grace_ms: 60_000,
			max_concurrent_materializations: 2,
			materialize_batch_chunks: 1024,
		}
	}
}
impl ForkConfig {
	pub fn validate(&self) -> Result<()> {
		if self.check_grace_ms == 0
			|| self.max_concurrent_materializations == 0
			|| self.materialize_batch_chunks == 0
		{
			bail!("fork cleanup age and materialization bounds must be positive");
		}
		Ok(())
	}
}

/// Optional overrides for the partition frontier and worker-owned callback pool.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct AlarmConfig {
	pub max_candidates: Option<u32>,
	pub max_operations: Option<u32>,
	pub max_callbacks: Option<u32>,
	pub page_size: Option<u32>,
	pub handler_timeout_ms: Option<u64>,
	pub io_timeout_ms: Option<u64>,
	pub retry_min_ms: Option<u64>,
	pub retry_max_ms: Option<u64>,
}
