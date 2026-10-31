//! Grant-owned volume cache and bounded handler tasks. Revocation cancels work immediately.
use super::{
	CommandRegistry, KvHandle, LockEntry, ReadHandle, ReadSource, RegisteredHandler,
	session::{WriterlessRead, WriterlessReadError},
};
use crate::kv_refusals::{authority_moved, overloaded, overloaded_after, unavailable};
use crate::tablet_maintainer_machine::{MaintainerSnapshot, state::MaintainerPhase};
use anyhow::{Result, ensure};
use bytes::Bytes;
use depot_client::{
	filesystem::{
		Config as FilesystemConfig, Error as FilesystemError, Filesystem, Kind, NoteExpiry,
		remote::RemoteColdStore, transport::VolumeJournal,
	},
	volume_requests::{Opened, VolumeRequests},
};
use depot_protocol::command::CommandTag;
use depot_volume::{
	ups::{UpsLogClient, client::UpsVolumeTransport},
	volume_client_machine::{
		runner::Volume,
		state::VolumeClientState,
		types::{ErrorDisposition, VolumeClientConfig},
	},
};
use parking_lot::Mutex;
use rivet_data::generated::depot_kv_command_v1::{
	ApplicationError, CommandResult, KvCommandRequest, KvCommandResponse, OverloadReason,
	UnavailableReason,
};
use rivet_data::generated::shared_depot_types_v1::ClientIdentity;
use rivet_env::{Env, TaskScope};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use rivet_state_machine::DeadlineQueue;
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	sync::{
		Arc, Weak,
		atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
	},
	time::Duration,
};
use tokio::sync::{OnceCell, watch};
use tokio_util::sync::CancellationToken;
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

#[derive(Clone)]
pub struct Config {
	/// The window of this worker's KV command subject; see `KvCommandConfig::request_capacity`.
	pub request_capacity: usize,
	pub max_open_volumes: usize,
	/// Upper bound per registered command class; waiters cannot consume event capacity.
	pub max_inflight_commands: usize,
	pub max_command_bytes: usize,
	/// Received bytes of the commands one host has admitted and not finished. A command that
	/// would exceed it is refused at once as unavailable, which callers retry.
	pub max_inflight_command_bytes: usize,
	pub volume_idle_close_ms: u64,
	pub evicted_idle_warn_ms: u64,
	/// Eviction closes this host runs at once; see `KvCommandConfig::max_eviction_closes`.
	pub max_eviction_closes: usize,
	pub filesystem: FilesystemConfig,
	/// The process cache budget shared with every other filesystem owner in this process.
	pub process_cache: Option<Arc<depot_client::filesystem::CacheBudget>>,
	/// Open command volumes across every tablet host of this worker, closing ones included.
	/// `None` leaves only each host's own `max_open_volumes`.
	pub worker_volumes: Option<Arc<OpenVolumeBudget>>,
	/// Floor for each registered command's handler timeout; `None` keeps the registered values.
	pub command_timeout_floor: Option<Duration>,
	/// Delay before retrying an idle cache release that could not run. Sweeps themselves
	/// run when the idle queue head is due.
	pub sweep_interval_ms: u64,
	/// Asks this worker's tablet manifest watcher to read the manifest now. The service
	/// signals it on every routing miss, so the watcher's interval is only a fallback.
	pub manifest_refresh: Option<watch::Sender<()>>,
	/// Codec for the payloads command volumes log; see `VolumeClientConfig::payload_codec`.
	pub payload_codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec,
	/// Log shard requests every command volume on this worker shares in flight to one storage
	/// worker before later ones queue for a batch.
	pub log_inflight_per_worker: usize,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			request_capacity: rivet_config::config::depot::KvCommandConfig::default()
				.request_capacity(),
			max_open_volumes: 128,
			max_inflight_commands: 256,
			max_command_bytes: 1024 * 1024,
			max_inflight_command_bytes: rivet_config::config::depot::KvCommandConfig::default()
				.max_inflight_command_bytes(),
			volume_idle_close_ms: 60_000,
			evicted_idle_warn_ms: 15_000,
			max_eviction_closes: rivet_config::config::depot::KvCommandConfig::default()
				.max_eviction_closes(),
			filesystem: FilesystemConfig::default(),
			process_cache: None,
			worker_volumes: None,
			command_timeout_floor: None,
			payload_codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec::Lz4,
			sweep_interval_ms: 20,
			manifest_refresh: None,
			log_inflight_per_worker: depot_volume::ups::DEFAULT_INFLIGHT_PER_WORKER,
		}
	}
}

/// Memory one open command volume holds in its worker, charged against the worker's
/// `depot.kv_commands.max_worker_open_volume_bytes`: its host slot, volume client and filesystem
/// owner, and the three log replicas and maintainer state its open loads. Measured at 58 to
/// 63 KB of requested heap by the `an_open_command_volume_costs_a_bounded_heap` test; the
/// charge leaves room for allocator overhead. Filesystem caches and command payloads are
/// bounded by their own budgets.
pub const OPEN_VOLUME_BYTES: u64 = 80 * 1024;

/// Open command volumes across every tablet host of one worker. Each host also bounds its own
/// volumes, but one process-wide cap keeps the worker's memory bounded however many tablets it
/// maintains. A volume holds its charge from its first open until its close finishes.
///
/// It publishes `KV_USAGE` and `KV_CAPACITY` as `volumes`, the cap that refuses opens first;
/// each host's own cap is `host_volumes`.
///
/// At a full budget, an open refused on one host may start an idle volume's close on another,
/// so a host whose volumes are all busy still makes room. The budget lists its hosts for that.
pub struct OpenVolumeBudget {
	open: AtomicUsize,
	capacity: usize,
	/// The hosts charging this budget, by registration id. Read only on the at-cap path.
	hosts: scc::HashMap<u64, Weak<VolumeHost>>,
	next_host: AtomicU64,
	/// Set on reaching four fifths of the cap and cleared below half of it, so the warning
	/// logs once per episode.
	near_cap: AtomicBool,
	_capacity_metric: IntGaugeGuard,
}

impl OpenVolumeBudget {
	pub fn new(capacity: usize) -> Arc<Self> {
		Arc::new(Self {
			open: AtomicUsize::new(0),
			capacity,
			hosts: scc::HashMap::new(),
			next_host: AtomicU64::new(0),
			near_cap: AtomicBool::new(false),
			_capacity_metric: crate::metrics::KV_CAPACITY
				.with_label_values(&["volumes"])
				.add_guard(capacity as i64),
		})
	}

	/// The worker's budget: `max_worker_open_volume_bytes` at [`OPEN_VOLUME_BYTES`] per volume,
	/// capped by the optional `max_worker_open_volumes` count.
	pub fn from_config(config: &rivet_config::config::depot::KvCommandConfig) -> Result<Arc<Self>> {
		let capacity = Self::capacity_of(config);
		ensure!(
			capacity > 0,
			"depot.kv_commands.max_worker_open_volume_bytes must hold at least one command volume of {OPEN_VOLUME_BYTES} bytes"
		);
		Ok(Self::new(capacity))
	}

	/// The open volumes `config` allows one worker.
	pub fn capacity_of(config: &rivet_config::config::depot::KvCommandConfig) -> usize {
		let by_bytes = config.max_worker_open_volume_bytes() / OPEN_VOLUME_BYTES;
		let by_bytes = usize::try_from(by_bytes).unwrap_or(usize::MAX);
		match config.max_worker_open_volumes {
			Some(count) => by_bytes.min(count),
			None => by_bytes,
		}
	}

	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Lists `host` among the hosts an at-cap open may evict from. Called from sync
	/// construction; no async code waits on this map.
	fn register(&self, host: Weak<VolumeHost>) -> u64 {
		let id = self.next_host.fetch_add(1, Ordering::Relaxed);
		let _ = self.hosts.insert_sync(id, host);
		id
	}

	fn deregister(&self, id: u64) {
		self.hosts.remove_sync(&id);
	}

	/// The live hosts of this budget with the volumes each holds charged, most first.
	fn hosts_by_charge(&self) -> Vec<(usize, Arc<VolumeHost>)> {
		let mut hosts = Vec::new();
		self.hosts.iter_sync(|_, host| {
			if let Some(host) = host.upgrade() {
				hosts.push((host.charged.load(Ordering::Acquire), host));
			}
			true
		});
		hosts.sort_by(|a, b| b.0.cmp(&a.0));
		hosts
	}

	fn try_reserve(self: &Arc<Self>, host: &Arc<AtomicUsize>) -> Option<OpenVolumeCharge> {
		let previous = self
			.open
			.try_update(Ordering::AcqRel, Ordering::Acquire, |open| {
				if open < self.capacity {
					Some(open + 1)
				} else {
					None
				}
			})
			.ok()?;
		let open = previous + 1;
		if open.saturating_mul(5) >= self.capacity.saturating_mul(4)
			&& !self.near_cap.swap(true, Ordering::AcqRel)
		{
			tracing::warn!(
				open,
				cap = self.capacity,
				"KV command volumes of this worker are near their cap; opens past it are refused as volumes_busy, raise depot.kv_commands.max_worker_open_volume_bytes if memory allows"
			);
		}
		host.fetch_add(1, Ordering::AcqRel);
		Some(OpenVolumeCharge {
			budget: self.clone(),
			host: host.clone(),
			_usage: crate::metrics::KV_USAGE
				.with_label_values(&["volumes"])
				.inc_guard(),
		})
	}
}

/// One volume's place in its worker's [`OpenVolumeBudget`], returned on drop.
struct OpenVolumeCharge {
	budget: Arc<OpenVolumeBudget>,
	/// The charging host's count of its charges, for the budget's fair share.
	host: Arc<AtomicUsize>,
	_usage: IntGaugeGuard,
}

impl Drop for OpenVolumeCharge {
	fn drop(&mut self) {
		self.host.fetch_sub(1, Ordering::AcqRel);
		let open = self.budget.open.fetch_sub(1, Ordering::AcqRel) - 1;
		if open.saturating_mul(2) < self.budget.capacity
			&& self.budget.near_cap.swap(false, Ordering::AcqRel)
		{
			tracing::info!(
				open,
				cap = self.budget.capacity,
				"KV command volumes of this worker recovered"
			);
		}
	}
}

#[derive(Clone)]
pub struct Factory {
	pub pubsub: PubSub,
	pub lifecycle: VolumeRequests,
	/// Must match the log service's reservation, including requests to our own worker.
	pub log_capacities: SubjectCapacities,
	/// Shared by every command volume's replica transport, so requests from many volumes to one
	/// storage worker coalesce.
	pub log_lanes: Arc<depot_volume::ups::LogRequestLanes>,
	pub registry: Arc<CommandRegistry>,
	pub config: Config,
	/// Signalled whenever a host's next sweep time moves earlier, including from open tasks
	/// outside the service loop, so the loop's sweep deadline is recomputed.
	pub idle_changed: Arc<tokio::sync::Notify>,
}

/// The speculation for a command that may create its volume, when its command allows it. It
/// runs under the command's own deadline tracker, so its writes stop when the caller's do.
fn speculation_for(
	request: &KvCommandRequest,
	command: &super::RegisteredCommand,
	commits: &Arc<super::CommandCommits>,
) -> Option<super::speculation::Speculation> {
	(request.create_if_missing && command.speculative_create).then(|| {
		super::speculation::Speculation {
			tag: CommandTag(request.tag as usize),
			operation_id: request.operation_id,
			body: request.body.clone().into(),
			commits: commits.clone(),
		}
	})
}

struct OpenVolume {
	filesystem: Arc<Filesystem>,
	ino: u64,
	volume: Volume,
	locked_ranges: Arc<Mutex<BTreeMap<Bytes, LockEntry>>>,
	shared: Arc<super::OpeningState>,
	/// The first command's speculative result, valid because this open created the volume
	/// with its writes. Taken once by that command. Forced-sync: read from a sync accessor.
	speculated: parking_lot::Mutex<Option<super::speculation::Speculated>>,
}

struct Slot {
	active: Arc<AtomicBool>,
	cancel: CancellationToken,
	opened: Arc<OnceCell<Result<OpenVolume, KvCommandResponse>>>,
	ready: watch::Receiver<bool>,
	inflight: usize,
	idle_since: u64,
	/// The shortest idle close among the commands admitted in this open.
	idle_close_ms: u64,
	/// This slot's entry in `Cache::idle`, set only while it is idle and ready.
	idle_at: Option<u64>,
	/// Consecutive transient open failures of this volume, carried into each reopen.
	open_failures: u32,
	/// Set only after a transient open failure: admission starts no reopen before it.
	retry_at: Option<u64>,
	/// The registered name of the command whose admission started this open, for metrics.
	command: &'static str,
	/// The open mode, fixed when the open starts: whether it creates a missing volume.
	create_if_absent: bool,
	/// Counts this slot on `KV_USAGE{host_volumes}` while it is in the cache. Taken when the slot
	/// leaves the cache to close, so a closing volume no longer counts as open.
	usage_metric: Option<IntGaugeGuard>,
	/// This volume's place in the worker's open volume budget, kept through restarts and
	/// returned once its close finishes.
	worker_charge: Option<OpenVolumeCharge>,
}

impl Slot {
	/// A slot whose open has not started yet. Its sender marks the open finished.
	fn new(
		now: u64,
		idle_close_ms: u64,
		open_failures: u32,
		command: &'static str,
		create_if_absent: bool,
	) -> (Self, watch::Sender<bool>) {
		let (ready, receiver) = watch::channel(false);
		let slot = Slot {
			active: Arc::new(AtomicBool::new(true)),
			cancel: CancellationToken::new(),
			opened: Arc::new(OnceCell::new()),
			ready: receiver,
			inflight: 0,
			idle_since: now,
			idle_close_ms,
			idle_at: None,
			open_failures,
			retry_at: None,
			command,
			create_if_absent,
			usage_metric: Some(
				crate::metrics::KV_USAGE
					.with_label_values(&["host_volumes"])
					.inc_guard(),
			),
			worker_charge: None,
		};
		(slot, ready)
	}

	/// Whether no command is admitted and the open has finished.
	fn settled(&self) -> bool {
		self.inflight == 0 && *self.ready.borrow()
	}
}

/// What admission does with an existing slot before admitting a command to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotAdmission {
	/// The slot serves the command, or its open is still running.
	Use,
	/// The slot is deactivated, fenced or its volume is missing: close it and reopen on a
	/// later command.
	Discard,
	/// A transient open failure is still inside its backoff.
	Backoff,
	/// A transient open failure's backoff has passed: start a fresh open in this slot.
	Reopen,
	/// The slot's open found the volume missing and this command may create it: start a
	/// creating open in this slot at once.
	Create,
}

/// How an open ended, for the reopen backoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenOutcome {
	Opened,
	/// The open may succeed later, so the next one waits for a backoff.
	Transient,
	/// The volume is missing, deleted or not owned here; admission discards the slot.
	Permanent,
}

impl OpenOutcome {
	fn of(result: &Result<OpenVolume, KvCommandResponse>) -> Self {
		match result {
			Ok(_) => OpenOutcome::Opened,
			Err(
				KvCommandResponse::Overloaded(_)
				| KvCommandResponse::Unavailable(_)
				| KvCommandResponse::Conflict,
			) => OpenOutcome::Transient,
			Err(
				KvCommandResponse::AuthorityMoved(_)
				| KvCommandResponse::Fenced
				| KvCommandResponse::Corrupt
				| KvCommandResponse::CommandResult(_)
				| KvCommandResponse::ApplicationError(_)
				| KvCommandResponse::VolumeNotFound
				| KvCommandResponse::VolumeDeleted
				| KvCommandResponse::NotMaintainer
				| KvCommandResponse::UnknownCommand
				| KvCommandResponse::CommandTooLarge,
			) => OpenOutcome::Permanent,
		}
	}
}

/// An ordinary open fails before any handler runs. Speculative opens may carry
/// the first command's application writes, so their uncertain result must survive.
fn readiness_failure(
	error: depot_volume::volume_client_machine::types::WriteError,
	initial_writes: bool,
) -> KvCommandResponse {
	use depot_volume::volume_client_machine::types::WriteError;
	use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse;
	match error {
		// The ordinary branch carries no initial application writes, and the
		// handler cannot run until readiness succeeds. Recovery can fail as the
		// assignment changes without making this command's outcome uncertain.
		WriteError::RecoveryFailed if !initial_writes => {
			overloaded(OverloadReason::VolumeTransition)
		}
		WriteError::QuorumUnavailable { token: None } if !initial_writes => {
			// A replica may unload during opening. No application payload was
			// supplied and no handler ran, so this attempt is safe to refuse.
			tracing::debug!(
				"ordinary command open lost loaded replica quorum; refusing before execution"
			);
			overloaded(OverloadReason::VolumeTransition)
		}
		WriteError::VolumeNotFound
		| WriteError::LifecycleRejected(VolumeRejectResponse::VolumeNotFound) => {
			KvCommandResponse::VolumeNotFound
		}
		WriteError::LifecycleRejected(VolumeRejectResponse::VolumeDeleted) => {
			KvCommandResponse::VolumeDeleted
		}
		// The volume's stored record is lost; nothing serves it until it is deleted, so the
		// edge must not retry the command into its deadline.
		WriteError::LifecycleRejected(VolumeRejectResponse::VolumeRecordMissing(_)) => {
			KvCommandResponse::Corrupt
		}
		// The open was never admitted, so the edge may send the command again after the hint.
		WriteError::Refused { retry_after_ms, .. }
		| WriteError::Overloaded {
			retry_after_ms,
			token: None,
		} => overloaded_after(OverloadReason::VolumeTransition, retry_after_ms),
		error => {
			tracing::warn!(
				?error,
				"command volume failed to become ready; outcome remains unknown"
			);
			unavailable(UnavailableReason::OpenFailed)
		}
	}
}

/// Reopen backoff after a transient open failure: 100 ms doubling to 5 s.
const REOPEN_BACKOFF_MIN_MS: u64 = 100;
const REOPEN_BACKOFF_MAX_MS: u64 = 5_000;

/// The backoff ceiling after `failures` consecutive failures. The delay is drawn from its
/// upper half so reopens of many volumes spread out.
fn reopen_backoff_ceiling_ms(failures: u32) -> u64 {
	let doublings = failures.saturating_sub(1).min(16);
	REOPEN_BACKOFF_MIN_MS
		.saturating_mul(1 << doublings)
		.min(REOPEN_BACKOFF_MAX_MS)
}

/// The open whose refusal started an eviction close, released when that close finishes.
struct RoomRequest {
	host: Weak<VolumeHost>,
	key: depot_types::VolumeKey,
}

#[derive(Default)]
struct Cache {
	volumes: BTreeMap<depot_types::VolumeKey, Slot>,
	/// Idle check times. A slot enters when it becomes idle with a finished open and keeps
	/// its entry while later commands run, so steady traffic never touches the queue. A
	/// due key is revalidated by the sweep, which reschedules a slot used since. The
	/// first settled slot in queue order is the eviction victim at capacity.
	idle: DeadlineQueue<depot_types::VolumeKey>,
	closing: BTreeSet<depot_types::VolumeKey>,
	/// Closes in `closing` that eviction started, bounded by `max_eviction_closes`.
	evicting: usize,
	/// Volumes of this host whose refused open started an eviction close, on this host or
	/// another, that has not finished, with when the claim lapses. A retry of such an open waits
	/// for that close instead of starting another, so one open evicts one volume however fast
	/// its caller retries. The close's end removes the entry; the lapse time covers a close
	/// that never ran, such as one cancelled by its host's revocation.
	room_requested: BTreeMap<depot_types::VolumeKey, u64>,
	/// Slots whose last open failed transiently. They are evicted before idle ready slots.
	failed: BTreeSet<depot_types::VolumeKey>,
	inflight: usize,
	/// Received bytes of the admitted commands, bounded by `max_inflight_command_bytes`.
	inflight_bytes: usize,
	by_command: BTreeMap<CommandTag, usize>,
	recent_short_evictions: VecDeque<u64>,
	churn_warning: bool,
	near_cap: [bool; 4],
}

impl Cache {
	/// Schedules `key`'s idle check from its slot, or cancels it while a command is admitted
	/// or the open is still running. A deactivated slot or a missing volume is due at once.
	/// A transient open failure is due one idle period after its backoff, so its failure count
	/// survives until the next reopen can use it. The open task schedules a slot that becomes
	/// ready with no admitted command.
	fn schedule_idle(&mut self, key: &depot_types::VolumeKey, now: u64, sweep_interval_ms: u64) {
		let Cache { volumes, idle, .. } = self;
		let Some(slot) = volumes.get_mut(key) else {
			return;
		};
		let at = idle_target(slot, now, sweep_interval_ms);
		match (slot.idle_at, at) {
			(old, Some(new)) => idle.replace(key.clone(), old, new),
			(Some(old), None) => {
				idle.cancel(key, old);
			}
			(None, None) => {}
		}
		slot.idle_at = at;
	}

	/// Records that a command on `key`'s slot finished at `now`. The last one to finish
	/// queues the slot's idle check only if it has none queued at or before its new time,
	/// so a volume in steady use is queued once, when it first goes idle.
	fn command_finished(&mut self, key: &depot_types::VolumeKey, now: u64, sweep_interval_ms: u64) {
		let Cache { volumes, idle, .. } = self;
		let Some(slot) = volumes.get_mut(key) else {
			return;
		};
		slot.inflight -= 1;
		if slot.inflight > 0 {
			return;
		}
		slot.idle_since = now;
		let Some(at) = idle_target(slot, now, sweep_interval_ms) else {
			return;
		};
		if slot.idle_at.is_some_and(|queued| queued <= at) {
			return;
		}
		idle.replace(key.clone(), slot.idle_at, at);
		slot.idle_at = Some(at);
	}

	/// When the sweep next has due work: the idle queue head, or the end of the churn
	/// window that clears the churn warning.
	fn next_sweep_at(&self) -> Option<u64> {
		let churn_ends_at = self
			.recent_short_evictions
			.front()
			.map(|time| time.saturating_add(60_000));
		match (self.idle.next_at(), churn_ends_at) {
			(Some(idle), Some(churn)) => Some(idle.min(churn)),
			(idle, churn) => idle.or(churn),
		}
	}
}

/// Why a due slot closes now, or `None` while it is busy or not idle long enough.
fn close_reason(slot: &Slot, now: u64) -> Option<CloseReason> {
	let settled = slot.inflight == 0 && *slot.ready.borrow();
	if !settled {
		None
	} else if !slot.active.load(Ordering::Acquire) {
		Some(CloseReason::Deactivated)
	} else if slot.opened.get().is_some_and(Result::is_err) {
		Some(CloseReason::FailedOpen)
	} else if now.saturating_sub(slot.idle_since) >= slot.idle_close_ms {
		Some(CloseReason::Idle)
	} else {
		None
	}
}

/// When `slot`'s next idle check is due, or `None` while a command is admitted or its open
/// is still running. Running the filesystem's note timer here drops the notes already due
/// and tells when the next one is.
fn idle_target(slot: &Slot, now: u64, sweep_interval_ms: u64) -> Option<u64> {
	if slot.inflight > 0 || !*slot.ready.borrow() {
		None
	} else if !slot.active.load(Ordering::Acquire) {
		Some(now)
	} else if let Some(retry_at) = slot.retry_at {
		Some(retry_at.saturating_add(slot.idle_close_ms))
	} else if slot.opened.get().is_some_and(Result::is_err) {
		Some(now)
	} else {
		// An idle volume drops the notes of its writes as their timers pass, then closes.
		let close_at = slot.idle_since.saturating_add(slot.idle_close_ms);
		let notes_at = match slot.opened.get() {
			Some(Ok(opened)) => match opened.filesystem.expire_notes(now) {
				NoteExpiry::Idle => None,
				NoteExpiry::Due(at) => Some(at),
				NoteExpiry::Busy => Some(now.saturating_add(sweep_interval_ms)),
			},
			Some(Err(_)) | None => None,
		};
		Some(notes_at.map_or(close_at, |at| at.min(close_at)))
	}
}

impl Cache {
	/// Removes a slot and its idle entry, so the queue only holds live slots.
	fn remove_slot(&mut self, key: &depot_types::VolumeKey) -> Option<Slot> {
		self.failed.remove(key);
		let slot = self.volumes.remove(key)?;
		if let Some(at) = slot.idle_at {
			self.idle.cancel(key, at);
		}
		Some(slot)
	}

	/// Decides how admission treats `key`'s existing slot at `now` for a command that may or
	/// may not create a missing volume.
	fn slot_admission(
		&self,
		key: &depot_types::VolumeKey,
		now: u64,
		create_if_missing: bool,
	) -> SlotAdmission {
		let Some(slot) = self.volumes.get(key) else {
			return SlotAdmission::Use;
		};
		if !slot.settled() {
			return SlotAdmission::Use;
		}
		if !slot.active.load(Ordering::Acquire) {
			return SlotAdmission::Discard;
		}
		match (slot.opened.get(), slot.retry_at) {
			(Some(Err(_)), Some(retry_at)) if now < retry_at => SlotAdmission::Backoff,
			(Some(Err(_)), Some(_)) => SlotAdmission::Reopen,
			(Some(Err(KvCommandResponse::VolumeNotFound)), None) if create_if_missing => {
				SlotAdmission::Create
			}
			(Some(Err(_)), None) => SlotAdmission::Discard,
			(Some(Ok(opened)), _) if opened.filesystem.is_fenced() => SlotAdmission::Discard,
			(Some(Ok(_)), _) | (None, _) => SlotAdmission::Use,
		}
	}

	/// Replaces a failed slot's outcome with a fresh open state, keeping its failure count.
	/// Returns the sender that marks the new open finished.
	fn restart_slot(
		&mut self,
		key: &depot_types::VolumeKey,
		now: u64,
		idle_close_ms: u64,
		command: &'static str,
		create_if_absent: bool,
	) -> Option<watch::Sender<bool>> {
		let failures = self.volumes.get(key)?.open_failures;
		let previous_idle_at = self.volumes.get(key)?.idle_at;
		if let Some(at) = previous_idle_at {
			self.idle.cancel(key, at);
		}
		self.failed.remove(key);
		let worker_charge = self.volumes.get_mut(key)?.worker_charge.take();
		let (mut slot, ready) = Slot::new(now, idle_close_ms, failures, command, create_if_absent);
		slot.worker_charge = worker_charge;
		self.volumes.insert(key.clone(), slot);
		Some(ready)
	}

	/// Records the outcome of the open that `opened` belongs to, if that open still owns
	/// `key`'s slot. A transient failure arms the reopen backoff; success resets it.
	/// `jitter(range)` returns a value in `0..=range`.
	fn record_open(
		&mut self,
		key: &depot_types::VolumeKey,
		opened: &Arc<OnceCell<Result<OpenVolume, KvCommandResponse>>>,
		outcome: OpenOutcome,
		now: u64,
		jitter: impl FnOnce(u64) -> u64,
	) {
		let Some(slot) = self.volumes.get_mut(key) else {
			return;
		};
		if !Arc::ptr_eq(&slot.opened, opened) {
			return;
		}
		match outcome {
			OpenOutcome::Opened => {
				slot.open_failures = 0;
				slot.retry_at = None;
			}
			OpenOutcome::Transient => {
				slot.open_failures = slot.open_failures.saturating_add(1);
				let ceiling = reopen_backoff_ceiling_ms(slot.open_failures);
				let delay = ceiling / 2 + jitter(ceiling / 2);
				slot.retry_at = Some(now.saturating_add(delay));
				self.failed.insert(key.clone());
			}
			OpenOutcome::Permanent => {}
		}
	}

	/// The slot to evict at capacity: a settled failed slot first, else the settled slot due
	/// to close first. Busy slots may still hold an entry from before they were used.
	fn eviction_victim(&self) -> Option<depot_types::VolumeKey> {
		let failed = self
			.failed
			.iter()
			.find(|key| self.volumes.get(*key).is_some_and(Slot::settled));
		if let Some(key) = failed {
			return Some(key.clone());
		}
		self.idle
			.iter()
			.map(|(_, key)| key)
			.find(|key| self.volumes.get(*key).is_some_and(Slot::settled))
			.cloned()
	}
}





pub struct VolumeHost {
	alarm_owner: std::sync::OnceLock<rivet_data::generated::shared_depot_alarms_v1::AlarmOwner>,
	factory: Factory,
	env: Arc<dyn Env>,
	cache: Mutex<Cache>,
	active: Arc<AtomicBool>,
	may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
	snapshot: watch::Receiver<MaintainerSnapshot>,
	tasks: TaskScope,
	/// Jitter for reopen backoff.
	rng: rivet_env::RngStream,
	/// `Cache::next_sweep_at`, or `u64::MAX` for none, published after every change under
	/// the cache lock so the tablet loop reads its deadline without taking the lock.
	next_sweep_at: AtomicU64,
	/// Volumes this host holds charged against the worker budget, closing ones included.
	charged: Arc<AtomicUsize>,
	/// This host's registration in the worker budget, removed on drop.
	budget_id: Option<u64>,
	/// The configured capacities on `KV_CAPACITY`, released with the host.
	_capacity_metrics: [IntGaugeGuard; 4],
}

impl VolumeHost {
	/// Membership changes visit this host's bounded cache once. The core snapshot
	/// remains authoritative; the cache never keeps a second assignment generation.
	pub(crate) fn refresh_replica_sets(&self) {
		let cache = self.lock_cache();
		for slot in cache.volumes.values() {
			self.offer_replica_set(slot);
		}
	}

	fn offer_replica_set(&self, slot: &Slot) {
		if let Some(Ok(opened)) = slot.opened.get() {
			opened
				.volume
				.handle()
				.observe_replica_set(self.snapshot.borrow().replica_set.clone());
		}
	}

	pub(crate) fn set_alarm_owner(
		&self,
		owner: rivet_data::generated::shared_depot_alarms_v1::AlarmOwner,
	) {
		let _ = self.alarm_owner.set(owner);
	}
	/// Takes the cache lock, recording how long the caller waited for it. An uncontended
	/// take records zero without reading the clock.
	fn lock_cache(&self) -> parking_lot::MutexGuard<'_, Cache> {
		if let Some(cache) = self.cache.try_lock() {
			crate::metrics::KV_HOST_LOCK_WAIT.observe(0.0);
			return cache;
		}
		#[allow(clippy::disallowed_methods, reason = "telemetry only")]
		let started = std::time::Instant::now();
		let cache = self.cache.lock();
		crate::metrics::KV_HOST_LOCK_WAIT.observe(started.elapsed().as_secs_f64());
		cache
	}

	pub fn new(
		mut factory: Factory,
		snapshot: watch::Receiver<MaintainerSnapshot>,
		authority: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Arc<Self>> {
		ensure!(
			factory.config.max_open_volumes > 0
				&& factory.config.max_inflight_commands > 0
				&& factory.config.max_command_bytes > 0
				&& factory.config.volume_idle_close_ms > 0
				&& factory.config.max_eviction_closes > 0,
			"KV command limits must be positive"
		);
		factory.config.max_command_bytes = factory
			.config
			.max_command_bytes
			.min(factory.pubsub.max_message_bytes());
		ensure!(
			factory.config.max_inflight_command_bytes >= factory.config.max_command_bytes,
			"KV in-flight command bytes must hold at least one largest command"
		);
		let active = Arc::new(AtomicBool::new(true));
		let serving = active.clone();
		let serving_phase = snapshot.clone();
		let capacity_metrics = [
			crate::metrics::KV_CAPACITY
				.with_label_values(&["host_volumes"])
				.add_guard(factory.config.max_open_volumes as i64),
			crate::metrics::KV_CAPACITY
				.with_label_values(&["commands"])
				.add_guard(Self::command_capacity_of(&factory) as i64),
			crate::metrics::KV_CAPACITY
				.with_label_values(&["bytes"])
				.add_guard(factory.config.max_inflight_command_bytes as i64),
			crate::metrics::KV_CAPACITY
				.with_label_values(&["eviction_closes"])
				.add_guard(factory.config.max_eviction_closes as i64),
		];
		let host = Arc::new_cyclic(|weak| Self {
			charged: Arc::new(AtomicUsize::new(0)),
			budget_id: factory
				.config
				.worker_volumes
				.as_ref()
				.map(|budget| budget.register(weak.clone())),
			alarm_owner: Default::default(),
			env: factory.pubsub.env().clone(),
			tasks: TaskScope::new(factory.pubsub.env().clone()),
			rng: factory.pubsub.env().rng("depot.kv.reopen_backoff"),
			factory,
			cache: Mutex::new(Cache::default()),
			next_sweep_at: AtomicU64::new(u64::MAX),
			active,
			may_serve: Arc::new(move || {
				serving.load(Ordering::Acquire)
					&& authority() && !matches!(
					serving_phase.borrow().phase,
					MaintainerPhase::Frozen
						| MaintainerPhase::Draining
						| MaintainerPhase::Fenced
						| MaintainerPhase::Stopped
				)
			}),
			snapshot,
			_capacity_metrics: capacity_metrics,
		});
		Ok(host)
	}

	/// Stop admission and cancel handlers before the maintainer waits on unrelated cold-store work.
	///
	/// Revocation releases every opened volume: its filesystem and volume client end here, and
	/// the next owner opens the volume again under a higher client epoch. Each filesystem is
	/// fenced as released before its handlers are cancelled, so a handler dropped while its
	/// commit is in flight is not reported as an abandoned write of a filesystem nobody else
	/// owns. Its caller is answered `authority_moved` and follows the new owner.
	pub fn revoke(&self) {
		self.active.store(false, Ordering::Release);
		let released: Vec<Arc<Filesystem>> = self
			.lock_cache()
			.volumes
			.values()
			.filter_map(|slot| match slot.opened.get() {
				Some(Ok(opened)) => Some(opened.filesystem.clone()),
				Some(Err(_)) | None => None,
			})
			.collect();
		for filesystem in released {
			filesystem.fence_with(depot_client::fences::FenceReason::OwnerReleased);
		}
		self.tasks.cancel();
		let mut cache = self.lock_cache();
		cache.volumes.clear();
		cache.failed.clear();
		cache.idle = DeadlineQueue::new();
		self.publish(&cache);
	}

	/// Publishes the next sweep time after a change under the cache lock, and wakes the
	/// tablet loop only if it moved earlier.
	fn publish(&self, cache: &Cache) {
		let next = cache.next_sweep_at().unwrap_or(u64::MAX);
		let previous = self.next_sweep_at.swap(next, Ordering::AcqRel);
		if next < previous {
			self.factory.idle_changed.notify_one();
		}
	}

	/// Joins cancelled handlers, without waiting for transaction or volume drain.
	pub async fn shutdown(&self) {
		self.revoke();
		self.tasks.shutdown().await;
	}

	/// Reports whether a close for `key` must not unload it because this host still owns it.
	///
	/// A raw volume client on this worker, such as lazy creation's cleanup entry, can close
	/// a volume that the host has since opened with its own client. That close must not
	/// unload replicas under the host's pending writes. The host retains the volume while an
	/// active slot is still opening, or has opened exactly this incarnation under a client
	/// identity different from `closing`. Eviction removes the slot before the host closes
	/// its own client, so that close still unloads normally.
	pub(crate) fn retains_volume(
		&self,
		key: &depot_types::VolumeLogKey,
		closing: Option<&ClientIdentity>,
	) -> bool {
		let cache = self.lock_cache();
		let Some(slot) = cache.volumes.get(&key.0.volume_key) else {
			return false;
		};
		if !slot.active.load(Ordering::Acquire) {
			return false;
		}

		match slot.opened.get() {
			None => true,
			Some(Err(_)) => false,
			Some(Ok(opened)) => {
				let handle = opened.volume.handle();
				let snapshot = handle.snapshot.borrow();
				let same_incarnation = snapshot.volume_log_key.as_deref() == Some(key);
				let other_client = closing != Some(&snapshot.client);
				same_incarnation && other_client
			}
		}
	}

	/// Deleting a volume invalidates cached snapshots as well as in-flight transactions.
	pub fn invalidate(&self, key: &depot_types::VolumeLogKey) {
		let now = self.env.clock().observe().map_or(0, |now| now.boot_ms);
		let mut cache = self.lock_cache();
		if let Some(slot) = cache.volumes.get(&key.0.volume_key)
			&& slot.opened.get().is_none_or(|result| {
				result.as_ref().map_or(true, |opened| {
					opened
						.volume
						.handle()
						.snapshot
						.borrow()
						.volume_log_key
						.as_deref() == Some(key)
				})
			}) {
			slot.active.store(false, Ordering::Release);
			slot.cancel.cancel();
			// An idle deactivated slot closes at the next sweep; a busy one when its last
			// command finishes.
			cache.schedule_idle(&key.0.volume_key, now, self.sweep_interval_ms());
			self.publish(&cache);
		}
	}

	/// When `sweep` next has due work: the idle queue head, or the end of the churn
	/// window that clears the churn warning. Reads the published time without locking.
	pub fn next_sweep_at(&self) -> Option<u64> {
		match self.next_sweep_at.load(Ordering::Acquire) {
			u64::MAX => None,
			at => Some(at),
		}
	}

	/// Expire only idle volumes. Closing runs outside the maintainer loop and never blocks a move.
	pub fn sweep(self: &Arc<Self>) {
		if !(self.may_serve)()
			|| matches!(
				self.snapshot.borrow().phase,
				MaintainerPhase::Fenced | MaintainerPhase::Stopped
			) {
			self.revoke();
			return;
		}
		let Ok(now) = self.env.clock().observe() else {
			return;
		};
		let mut cache = self.lock_cache();
		while cache
			.recent_short_evictions
			.front()
			.is_some_and(|time| now.boot_ms.saturating_sub(*time) >= 60_000)
		{
			cache.recent_short_evictions.pop_front();
		}
		if cache.recent_short_evictions.is_empty() {
			cache.churn_warning = false;
		}
		// Only due keys are visited. Each is revalidated against its slot, and one used or
		// lowered since is rescheduled from its current fields.
		let due = cache.idle.pop_due(now.boot_ms, usize::MAX);
		depot_client::metrics::record_idle_visits("volume_host_sweep", due.len());
		for (_, key) in due {
			let Some(slot) = cache.volumes.get_mut(&key) else {
				continue;
			};
			slot.idle_at = None;
			// Missing volumes and failed opens have no filesystem worth caching.
			if let Some(reason) = close_reason(slot, now.boot_ms) {
				if let Some(slot) = cache.remove_slot(&key) {
					self.close_idle(&mut cache, key, slot, reason, None);
				}
				continue;
			}
			cache.schedule_idle(&key, now.boot_ms, self.sweep_interval_ms());
		}
		self.observe(&mut cache);
		self.publish(&cache);
	}

	/// Closes every open volume that runs no admitted command now, as its idle close would
	/// later, and returns how many it closed. An operator uses it to make a tablet's data cold
	/// without waiting out `volume_idle_close_ms`. A closed volume reopens on its next command.
	/// A slot in reopen backoff stays, so its failure count still spaces the next reopen. The
	/// visit is bounded by `max_open_volumes`.
	pub fn close_idle_now(self: &Arc<Self>) -> usize {
		if !(self.may_serve)() {
			return 0;
		}
		let mut cache = self.lock_cache();
		let idle = cache
			.volumes
			.iter()
			.filter(|(_, slot)| {
				slot.inflight == 0 && *slot.ready.borrow() && slot.retry_at.is_none()
			})
			.map(|(key, _)| key.clone())
			.collect::<Vec<_>>();
		let mut closed = 0;
		for key in idle {
			if let Some(slot) = cache.remove_slot(&key) {
				self.close_idle(&mut cache, key, slot, CloseReason::Operator, None);
				closed += 1;
			}
		}
		self.observe(&mut cache);
		self.publish(&cache);
		closed
	}

	fn close_idle(
		self: &Arc<Self>,
		cache: &mut Cache,
		key: depot_types::VolumeKey,
		mut slot: Slot,
		reason: CloseReason,
		requester: Option<RoomRequest>,
	) {
		crate::metrics::KV_VOLUME_CLOSES
			.with_label_values(&[slot.command, reason.label()])
			.inc();
		cache.closing.insert(key.clone());
		let evicting = reason == CloseReason::Evicted;
		let eviction_usage = if evicting {
			cache.evicting += 1;
			Some(
				crate::metrics::KV_USAGE
					.with_label_values(&["eviction_closes"])
					.inc_guard(),
			)
		} else {
			None
		};
		let host = Arc::downgrade(self);
		drop(slot.usage_metric.take());
		self.tasks.spawn(async move {
			if let Ok(cell) = Arc::try_unwrap(slot.opened)
				&& let Some(Ok(opened)) = cell.into_inner()
			{
				// An uncertain write is recovered by the next opening's higher claim.
				if !opened.filesystem.is_fenced() {
					let _ = opened.filesystem.close().await;
				}

				let _ = opened.volume.shutdown().await;
			}
			if let Some(host) = host.upgrade() {
				let mut cache = host.lock_cache();
				cache.closing.remove(&key);
				if evicting {
					cache.evicting -= 1;
				}
			}
			drop(eviction_usage);
			// The room is free now, so the next refusal of the requesting open may evict again.
			if let Some(requester) = requester
				&& let Some(host) = requester.host.upgrade()
			{
				host.lock_cache().room_requested.remove(&requester.key);
			}
		});
	}

	/// Admission reserves a slot before any await, so concurrent first commands share one open.
	///
	/// A read command for a volume that has no slot is admitted without one; it runs on a
	/// maintainer read session and never opens the writer.
	///
	/// `bytes` is the command's received wire size.
	fn admit(
		self: &Arc<Self>,
		request: &KvCommandRequest,
		bytes: usize,
		commits: &Arc<super::CommandCommits>,
	) -> Result<Admission, KvCommandResponse> {
		// Reject a stale grant or frozen tablet before reserving any runtime resources.
		if self.snapshot.borrow().phase != MaintainerPhase::Serving {
			return Err(authority_moved());
		}
		if !(self.may_serve)() {
			return Err(KvCommandResponse::NotMaintainer);
		}

		// Check the full wire size before opening a database.
		if bytes > self.factory.config.max_command_bytes {
			return Err(KvCommandResponse::CommandTooLarge);
		}
		let now = self
			.env
			.clock()
			.observe()
			.map_err(|_| unavailable(UnavailableReason::Internal))?
			.boot_ms;

		// Reserve both budgets under one lock; closing volumes keep their capacity.
		let mut worker_full = false;
		let admitted = {
			let mut cache = self.lock_cache();
			let admitted =
				self.admit_locked(&mut cache, request, bytes, now, commits, &mut worker_full);
			self.publish(&cache);
			admitted
		};
		// Another host's cache lock is taken only after this one is released, so two hosts
		// evicting from each other cannot deadlock. `admit_locked` already recorded the room
		// request, so a close that finishes at once still releases it.
		if worker_full && !self.free_worker_capacity(now, &request.volume_key) {
			self.lock_cache().room_requested.remove(&request.volume_key);
		}
		admitted
	}

	/// Starts one idle volume's close for `key`'s open, refused at the full worker budget, and
	/// returns whether one started. A host below its fair share of the budget takes from the
	/// hosts holding the most first, then from itself. A host at or above it evicts its own
	/// volumes first, then only from hosts also above their share, so one hot tablet can
	/// neither keep the whole budget from the others nor take their volumes. Each host keeps
	/// its own eviction close limit.
	fn free_worker_capacity(self: &Arc<Self>, now: u64, key: &depot_types::VolumeKey) -> bool {
		let Some(budget) = &self.factory.config.worker_volumes else {
			return false;
		};
		let hosts = budget.hosts_by_charge();
		let fair_share = budget.capacity() / hosts.len().max(1);
		let own_first = self.charged.load(Ordering::Acquire) >= fair_share;
		let others = hosts
			.iter()
			.filter(|(_, host)| !Arc::ptr_eq(host, self))
			.filter(|(charged, _)| !own_first || *charged > fair_share)
			.map(|(_, host)| host);
		let candidates: Vec<&Arc<VolumeHost>> = if own_first {
			std::iter::once(self).chain(others).collect()
		} else {
			others.chain(std::iter::once(self)).collect()
		};
		for host in candidates {
			let requester = RoomRequest {
				host: Arc::downgrade(self),
				key: key.clone(),
			};
			if host.evict_one(now, requester) {
				let result = if Arc::ptr_eq(host, self) {
					"own_host"
				} else {
					"other_host"
				};
				crate::metrics::KV_WORKER_BUDGET_EVICTIONS
					.with_label_values(&[result])
					.inc();
				return true;
			}
		}
		crate::metrics::KV_WORKER_BUDGET_EVICTIONS
			.with_label_values(&["none"])
			.inc();
		false
	}

	/// Starts closing this host's eviction victim unless it is already at its eviction close
	/// limit or has no settled volume. Returns whether a close started.
	fn evict_one(self: &Arc<Self>, now: u64, requester: RoomRequest) -> bool {
		let mut cache = self.lock_cache();
		let started = self.evict_locked(&mut cache, now, requester);
		if started {
			self.observe(&mut cache);
			self.publish(&cache);
		}
		started
	}

	/// The part of `evict_one` that runs under the cache lock. A revoked host starts no close:
	/// its task scope is cancelled, so the close would never run.
	fn evict_locked(self: &Arc<Self>, cache: &mut Cache, now: u64, requester: RoomRequest) -> bool {
		if !self.active.load(Ordering::Acquire)
			|| cache.evicting >= self.factory.config.max_eviction_closes
		{
			return false;
		}
		let Some(victim) = cache.eviction_victim() else {
			return false;
		};
		let Some(slot) = cache.remove_slot(&victim) else {
			return false;
		};
		let idle = now.saturating_sub(slot.idle_since);
		self.evicted(cache, now, idle);
		self.close_idle(cache, victim, slot, CloseReason::Evicted, Some(requester));
		true
	}

	/// The part of `admit` that runs under the cache lock. Sets `worker_full` when it refused
	/// the open at the full worker budget, so `admit` frees capacity after the lock.
	fn admit_locked(
		self: &Arc<Self>,
		cache: &mut Cache,
		request: &KvCommandRequest,
		bytes: usize,
		now: u64,
		commits: &Arc<super::CommandCommits>,
		worker_full: &mut bool,
	) -> Result<Admission, KvCommandResponse> {
		// `revoke` clears the slots under this lock after fencing their filesystems outside
		// it. A command admitted in between would open a slot that nothing fences or closes,
		// so a revoked host admits nothing: its caller follows the tablet's next owner.
		if !self.active.load(Ordering::Acquire) {
			return Err(authority_moved());
		}
		if cache.closing.contains(&request.volume_key) {
			return Err(overloaded(OverloadReason::VolumeTransition));
		}
		let tag = CommandTag(request.tag as usize);
		let Some(command) = self.factory.registry.commands.get(&tag) else {
			return Err(KvCommandResponse::UnknownCommand);
		};
		let limit = command
			.concurrency
			.min(self.factory.config.max_inflight_commands);
		if cache.by_command.get(&tag).copied().unwrap_or(0) >= limit {
			return Err(overloaded(OverloadReason::CommandCapacity));
		}
		if cache.inflight_bytes + bytes > self.factory.config.max_inflight_command_bytes {
			return Err(overloaded(OverloadReason::CommandBytes));
		}

		// A read of a volume that is not open here takes no slot and starts no volume client.
		let is_read = match &command.handler {
			RegisteredHandler::Read(_) => true,
			RegisteredHandler::Write(_) => false,
		};
		if is_read && !request.create_if_missing && !cache.volumes.contains_key(&request.volume_key)
		{
			cache.inflight += 1;
			cache.inflight_bytes += bytes;
			*cache.by_command.entry(tag).or_default() += 1;
			self.observe(cache);
			return Ok(Admission {
				tag,
				ready: watch::channel(true).1,
				active: Arc::new(AtomicBool::new(true)),
				cancel: CancellationToken::new(),
				host: self.clone(),
				key: request.volume_key.clone(),
				opened: None,
				bytes,
				slotted: false,
				_usage_metrics: Admission::usage_metrics(bytes),
			});
		}

		// An uncertain filesystem must reopen before another command can use its cached state.
		// A transient open failure reopens in place once its backoff passes.
		// A missing volume that this command may create opens again at once in create mode.
		match cache.slot_admission(&request.volume_key, now, request.create_if_missing) {
			SlotAdmission::Use => {}
			SlotAdmission::Discard => {
				if let Some(slot) = cache.remove_slot(&request.volume_key) {
					self.close_idle(
						cache,
						request.volume_key.clone(),
						slot,
						CloseReason::Discarded,
						None,
					);
				}
				return Err(overloaded(OverloadReason::VolumeTransition));
			}
			SlotAdmission::Backoff => return Err(overloaded(OverloadReason::VolumeTransition)),
			SlotAdmission::Reopen | SlotAdmission::Create => {
				let idle_close_ms = self.factory.config.volume_idle_close_ms;
				if let Some(ready) = cache.restart_slot(
					&request.volume_key,
					now,
					idle_close_ms,
					command_label(tag),
					request.create_if_missing,
				) && let Some(slot) = cache.volumes.get(&request.volume_key)
				{
					self.start_open(
						request.volume_key.clone(),
						slot,
						ready,
						speculation_for(request, command, commits),
					);
				}
			}
		}
		// Best-effort offers can meet a full client inbox. A real command retries
		// the current published membership without another timer or network request.
		if let Some(slot) = cache.volumes.get(&request.volume_key) {
			self.offer_replica_set(slot);
		}

		// Evict only an idle volume, and finish its close before using that capacity. Each
		// refused open starts at most one close, up to `max_eviction_closes` at once.
		if !cache.volumes.contains_key(&request.volume_key) {
			// At a full cap, an open whose earlier refusal already started a close waits for it.
			let room_lapses_at =
				now.saturating_add(self.factory.config.filesystem.request_timeout_ms);
			let room_claimed = match cache.room_requested.get(&request.volume_key) {
				Some(lapses_at) => now < *lapses_at,
				None => false,
			};
			if cache.volumes.len() + cache.closing.len() >= self.factory.config.max_open_volumes {
				if room_claimed {
					return Err(overloaded(OverloadReason::VolumesBusy));
				}
				let requester = RoomRequest {
					host: Arc::downgrade(self),
					key: request.volume_key.clone(),
				};
				if self.evict_locked(cache, now, requester) {
					cache
						.room_requested
						.insert(request.volume_key.clone(), room_lapses_at);
				}
				return Err(overloaded(OverloadReason::VolumesBusy));
			}
			let worker_charge = match &self.factory.config.worker_volumes {
				Some(budget) => match budget.try_reserve(&self.charged) {
					Some(charge) => Some(charge),
					None if room_claimed => {
						return Err(overloaded(OverloadReason::VolumesBusy));
					}
					None => {
						// Recorded before any close can start, so its finish always releases it.
						cache
							.room_requested
							.insert(request.volume_key.clone(), room_lapses_at);
						*worker_full = true;
						return Err(overloaded(OverloadReason::VolumesBusy));
					}
				},
				None => None,
			};
			let (mut slot, ready) = Slot::new(
				now,
				self.factory.config.volume_idle_close_ms,
				0,
				command_label(tag),
				request.create_if_missing,
			);
			self.start_open(
				request.volume_key.clone(),
				&slot,
				ready,
				speculation_for(request, command, commits),
			);
			slot.worker_charge = worker_charge;
			cache.volumes.insert(request.volume_key.clone(), slot);
			cache.room_requested.remove(&request.volume_key);
		}

		// The admission guard releases these counters on success, error or cancellation.
		cache.inflight += 1;
		*cache.by_command.entry(tag).or_default() += 1;
		let idle_close_ms = command
			.idle_close
			.map_or(self.factory.config.volume_idle_close_ms, |idle_close| {
				u64::try_from(idle_close.as_millis()).unwrap_or(u64::MAX)
			});
		let Some(slot) = cache.volumes.get_mut(&request.volume_key) else {
			return Err(overloaded(OverloadReason::VolumeTransition));
		};
		slot.inflight += 1;
		slot.idle_close_ms = slot.idle_close_ms.min(idle_close_ms);
		let opened = slot.opened.clone();
		let active = slot.active.clone();
		let cancel = slot.cancel.clone();
		let ready = slot.ready.clone();
		cache.inflight_bytes += bytes;
		// The slot's idle entry, if any, stays queued while the command runs; the sweep
		// finds it busy and reschedules it, so steady traffic never touches the queue.
		self.observe(cache);
		Ok(Admission {
			tag,
			ready,
			active,
			cancel,
			host: self.clone(),
			key: request.volume_key.clone(),
			opened: Some(opened),
			bytes,
			slotted: true,
			_usage_metrics: Admission::usage_metrics(bytes),
		})
	}

	/// Starts a bounded task and returns its reply immediately to the service loop.
	///
	/// `request_bytes` is the command's received wire size, which bounds it against
	/// `max_command_bytes` without encoding it again.
	pub fn submit(
		self: &Arc<Self>,
		request: KvCommandRequest,
		request_bytes: usize,
		deadline_boot_ms: u64,
	) -> tokio::sync::oneshot::Receiver<KvCommandResponse> {
		let (send, receive) = tokio::sync::oneshot::channel();
		if request.operation_id == [0; 16] {
			let _ = send.send(KvCommandResponse::ApplicationError(ApplicationError {
				group: "depot".into(),
				code: "invalid_operation_id".into(),
				message: "operation id must not be nil".into(),
			}));
			return receive;
		}
		let Some(command) = self
			.factory
			.registry
			.commands
			.get(&CommandTag(request.tag as usize))
		else {
			let _ = send.send(KvCommandResponse::UnknownCommand);
			return receive;
		};
		let timeout = self.factory.config.command_timeout(command.timeout);
		let Ok(local_deadline) = rivet_env::deadline_after(self.env.clock().as_ref(), timeout)
		else {
			let _ = send.send(unavailable(UnavailableReason::Internal));
			return receive;
		};
		let deadline = deadline_boot_ms.min(local_deadline);
		if self
			.env
			.clock()
			.observe()
			.map_or(true, |now| now.boot_ms >= deadline)
		{
			let _ = send.send(unavailable(UnavailableReason::TimedOut));
			return receive;
		}
		// The caller's deadline tracker. A speculative run of this command that its admission
		// starts shares it, so nothing it has not dispatched starts after the caller timed out.
		let commits = Arc::new(super::CommandCommits::default());
		let admission = match self.admit(&request, request_bytes, &commits) {
			Ok(admission) => admission,
			Err(error) => {
				tracing::debug!(?error, tag = request.tag, "KV command admission rejected");
				let _ = send.send(error);
				return receive;
			}
		};
		let host = self.clone();
		self.tasks.spawn(async move {
			let clock = host.env.clock();
			let work = host.execute(&admission, request, deadline, commits.clone());
			let mut work = std::pin::pin!(work);
			let finished = rivet_env::select_biased!("volume deletion cancels handlers before another poll",
				_ = admission.cancel.cancelled() => Some(authority_moved()),
				result = rivet_env::timeout_at(clock.as_ref(), deadline, &mut work) => result.ok(),
			);
			let timed_out = finished.is_none();
			let response = finished.unwrap_or_else(|| unavailable(UnavailableReason::TimedOut));
			let response = if (host.may_serve)() && admission.active.load(Ordering::Acquire) {
				response
			} else {
				authority_moved()
			};
			// The transport limit includes the response envelope, not only the handler's body.
			let response = match rivet_data::versioned::DepotKvCommandResponseData::serialize_with_embedded_version(&response, rivet_data::DEPOT_KV_COMMAND_VERSION) {
				Ok(bytes) if bytes.len() <= host.factory.config.max_command_bytes => response,
				Ok(_) => KvCommandResponse::CommandTooLarge,
				Err(_) => unavailable(UnavailableReason::Internal),
			};
			let _ = send.send(response);
			// The caller has its answer, but a commit the handler already dispatched keeps
			// running under the filesystem's own deadline and its admission until it settles.
			// Dropping it would abandon the write and fence the filesystem, though its unknown
			// outcome leaves the filesystem serving when the filesystem itself gives up on it.
			// Nothing the handler had not dispatched may start now, and the handler is dropped
			// once no commit is in flight, so the admission is held past the deadline for at most
			// one filesystem request timeout.
			if timed_out {
				commits.expire();
				rivet_env::select_biased!("volume deletion cancels handlers before another poll",
					_ = admission.cancel.cancelled() => {},
					_ = &mut work => {},
					_ = commits.settled() => {},
				);
			}
		});
		receive
	}

	async fn execute(
		self: &Arc<Self>,
		admission: &Admission,
		request: KvCommandRequest,
		deadline: u64,
		commits: Arc<super::CommandCommits>,
	) -> KvCommandResponse {
		if admission.slotted {
			return self.execute_with_writer(admission, request, commits).await;
		}
		let command_label = command_label(CommandTag(request.tag as usize));
		let command = &self.factory.registry.commands[&CommandTag(request.tag as usize)];
		let RegisteredHandler::Read(handler) = &command.handler else {
			return unavailable(UnavailableReason::Internal);
		};
		let read = ReadHandle {
			source: ReadSource::Writerless(Arc::new(WriterlessRead::new(
				Arc::new(self.factory.lifecycle.clone()),
				request.volume_key.clone(),
				b"kv",
				deadline,
			))),
			volume_key: request.volume_key,
		};
		let handler_timer = crate::metrics::KV_COMMAND_PHASE_DURATION
			.with_label_values(&[command_label, "handler"])
			.start_timer();
		let result = handler(read, request.body.to_vec()).await;
		handler_timer.observe_duration();

		// A refused point read fails the handler. A read never opens the writer: that would
		// turn read pressure into durable claims, so the refusal is retryable at the edge,
		// whether the maintainer is out of read capacity or the volume has a pending lifecycle
		// request.
		let refused = result
			.as_ref()
			.err()
			.and_then(|error| error.downcast_ref::<WriterlessReadError>())
			.copied();
		let (label, response) = match refused {
			Some(WriterlessReadError::NotFound) => ("not_found", KvCommandResponse::VolumeNotFound),
			Some(WriterlessReadError::Overloaded) => (
				"overloaded",
				overloaded(OverloadReason::FilesystemOverloaded),
			),
			Some(WriterlessReadError::TooLarge) => {
				("too_large", KvCommandResponse::CommandTooLarge)
			}
			Some(WriterlessReadError::Unavailable) => (
				"unavailable",
				overloaded(OverloadReason::FilesystemOverloaded),
			),
			None => ("served", self.command_response(command_label, result)),
		};
		crate::metrics::KV_WRITERLESS_READS
			.with_label_values(&[command_label, label])
			.inc();
		response
	}

	async fn execute_with_writer(
		&self,
		admission: &Admission,
		request: KvCommandRequest,
		commits: Arc<super::CommandCommits>,
	) -> KvCommandResponse {
		let authority = self.may_serve.clone();
		let active = admission.active.clone();
		let may_serve: Arc<dyn Fn() -> bool + Send + Sync> =
			Arc::new(move || active.load(Ordering::Acquire) && authority());
		// Opening belongs to the host: a caller's deadline never restarts volume recovery.
		let command_label = command_label(CommandTag(request.tag as usize));
		let open_wait = crate::metrics::KV_COMMAND_PHASE_DURATION
			.with_label_values(&[command_label, "open_wait"])
			.start_timer();
		let mut ready = admission.ready.clone();
		if ready.wait_for(|ready| *ready).await.is_err() {
			return overloaded(OverloadReason::VolumeTransition);
		}
		open_wait.observe_duration();
		let opened = match admission
			.opened
			.as_ref()
			.expect("active admission holds its volume")
			.get()
			.expect("ready volume has an outcome")
		{
			Ok(opened) => opened,
			// This command joined an open that could not create the volume. Retrying admits it
			// again, and admission starts a creating open in the same slot.
			Err(KvCommandResponse::VolumeNotFound) if request.create_if_missing => {
				return overloaded(OverloadReason::VolumeTransition);
			}
			Err(error) => return error.clone(),
		};
		// The command that ran speculatively before this open created the volume answers with
		// that result; its writes are already committed.
		let speculated = {
			let mut speculated = opened.speculated.lock();
			match speculated.as_ref() {
				Some(result) if result.operation_id == request.operation_id => speculated.take(),
				Some(_) | None => None,
			}
		};
		// Once the first caller timed out, the command's later work may have been refused, so a
		// retry runs the command again here instead of collecting what remained of its run.
		let speculated = speculated.filter(|speculated| !speculated.caller_timed_out);
		if let Some(speculated) = speculated {
			crate::metrics::KV_SPECULATIVE_CREATES.inc();
			return self.command_response(command_label, speculated.result);
		}
		let command = &self.factory.registry.commands[&CommandTag(request.tag as usize)];
		let kv = KvHandle {
			alarm_owner: self.alarm_owner.get().cloned(),
			filesystem: opened.filesystem.clone(),
			ino: opened.ino,
			may_serve,
			operation_id: request.operation_id,
			volume_key: request.volume_key,
			locked_ranges: opened.locked_ranges.clone(),
			shared: opened.shared.clone(),
			commits,
			speculative: None,
			versions: self.factory.pubsub.topology().clone(),
		};
		let handler_timer = crate::metrics::KV_COMMAND_PHASE_DURATION
			.with_label_values(&[command_label, "handler"])
			.start_timer();
		let result = command
			.handler
			.run_with_writer(kv, request.body.to_vec())
			.await;
		handler_timer.observe_duration();
		// The filesystem already retried refreshing its cold snapshot until the deadline, so a
		// `TooOld` that reaches the handler means this opening cannot catch up in place, for
		// example after its read session expired past retention. Fence it so the next command
		// reopens the volume from the log instead of every command failing the same way.
		if let Err(error) = &result
			&& matches!(
				error.downcast_ref::<FilesystemError>(),
				Some(FilesystemError::TooOld)
			) {
			tracing::warn!(
				command = command_label,
				"command volume filesystem cannot refresh its cold view; reopening"
			);
			opened
				.filesystem
				.fence_with(depot_client::fences::FenceReason::CommandColdTooOld);
		}
		self.command_response(command_label, result)
	}

	/// Maps a handler result to its wire response.
	fn command_response(
		&self,
		command: &'static str,
		result: Result<Vec<u8>>,
	) -> KvCommandResponse {
		match result {
			Ok(body) if body.len() > self.factory.config.max_command_bytes => {
				KvCommandResponse::CommandTooLarge
			}
			Ok(body) => KvCommandResponse::CommandResult(CommandResult { body: body.into() }),
			Err(error)
				if matches!(
					error.downcast_ref::<FilesystemError>(),
					Some(FilesystemError::TooLarge)
				) =>
			{
				KvCommandResponse::CommandTooLarge
			}
			Err(error)
				if error
					.downcast_ref::<FilesystemError>()
					.is_some_and(|error| {
						matches!(
							error,
							FilesystemError::Overloaded
								| FilesystemError::Conflict
								| FilesystemError::TooOld
								| FilesystemError::Fenced
								| FilesystemError::Lagging
								| FilesystemError::Unavailable
								| FilesystemError::ColdReadRefused
								| FilesystemError::DeadlineExceeded
								| FilesystemError::Log(_)
								| FilesystemError::Corrupt
								| FilesystemError::RecordMissing { .. }
						)
					}) =>
			{
				error.downcast_ref::<FilesystemError>().map_or(
					unavailable(UnavailableReason::StorageFailed),
					filesystem_refusal,
				)
			}
			Err(error) => {
				let extracted = rivet_error::RivetError::extract(&error);
				// An unstructured handler error reaches the caller only as `core.internal_error`,
				// so its cause is logged here, where it is still known.
				if extracted.group() == "core" && extracted.code() == "internal_error" {
					tracing::warn!(
						command,
						?error,
						"depot command failed with an unstructured error; the caller sees core.internal_error"
					);
				}
				let error = extracted;
				KvCommandResponse::ApplicationError(ApplicationError {
					group: error.group().to_string().into(),
					code: error.code().to_string().into(),
					message: error.message().to_string().into(),
				})
			}
		}
	}

	/// One host-owned open outlives individual callers, but ends on deletion or grant revocation.
	fn start_open(
		self: &Arc<Self>,
		key: depot_types::VolumeKey,
		slot: &Slot,
		ready: watch::Sender<bool>,
		speculation: Option<super::speculation::Speculation>,
	) {
		let host = self.clone();
		let idle_key = key.clone();
		let command = slot.command;
		let create_if_absent = slot.create_if_absent;
		let opened = slot.opened.clone();
		let active = slot.active.clone();
		let cancel = slot.cancel.clone();
		let authority = self.may_serve.clone();
		let may_serve: Arc<dyn Fn() -> bool + Send + Sync> =
			Arc::new(move || active.load(Ordering::Acquire) && authority());
		self.tasks.spawn(async move {
			let clock = host.env.clock();
			let timeout = Duration::from_millis(host.factory.config.filesystem.request_timeout_ms);
			let result = rivet_env::select_biased!("deletion cancels opening before another poll",
				_ = cancel.cancelled() => Err(overloaded(OverloadReason::VolumeTransition)),
				result = rivet_env::timeout(clock.as_ref(), timeout, host.open(key, may_serve, command, create_if_absent, speculation)) => result.unwrap_or_else(|_| Err(unavailable(UnavailableReason::TimedOut))),
			);
			crate::metrics::KV_VOLUME_OPENS
				.with_label_values(&[command, open_result_label(&result)])
				.inc();
			let outcome = OpenOutcome::of(&result);
			// Record the outcome before admission can observe the finished open.
			let now = clock.observe().map_or(0, |now| now.boot_ms);
			let mut rng = host.rng.clone();
			let mut cache = host.lock_cache();
			let _ = opened.set(result);
			if let Some(slot) = cache.volumes.get(&idle_key)
				&& Arc::ptr_eq(&slot.opened, &opened)
			{
				// A membership effect may have visited this still-opening slot.
				// Publish and offer under its same lock before waking callers.
				host.offer_replica_set(slot);
			}
			cache.record_open(&idle_key, &opened, outcome, now, |range| {
				rng.gen_range(0..range + 1)
			});
			// Release the task's reference before making an idle slot eligible for eviction.
			drop(opened);
			ready.send_replace(true);
			// Every caller may have left while the open ran; the slot is then idle now.
			cache.schedule_idle(&idle_key, now, host.sweep_interval_ms());
			host.publish(&cache);
		});
	}

	async fn open(
		&self,
		key: depot_types::VolumeKey,
		may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
		command: &'static str,
		create_if_absent: bool,
		speculation: Option<super::speculation::Speculation>,
	) -> Result<OpenVolume, KvCommandResponse> {
		// A speculative first command of a volume this open may create runs against an empty
		// volume first, so its writes ride the creating frame.
		let speculative = match (speculation, create_if_absent) {
			(Some(speculation), true) => self.speculate(&key, speculation, may_serve.clone()).await,
			(Some(_), false) | (None, _) => None,
		};
		let tablet = self
			.factory
			.lifecycle
			.tablet(&key)
			.map_err(|_| KvCommandResponse::NotMaintainer)?;
		// The host already bounds the whole open by the filesystem request timeout. A shorter
		// client wait would abandon a slow recovery and restart it on the next command.
		let request_timeout_ms = self.factory.config.filesystem.request_timeout_ms;
		// The host stops waiting at each command's own deadline, so the client's quorum wait
		// only needs a ceiling no shorter than the longest command deadline. A shorter fixed
		// wait would end a write or barrier while its command still had time, and on a fresh
		// volume it also abandons the creating claim entry the first write rides behind.
		let longest_command_ms = self
			.factory
			.registry
			.commands
			.values()
			.map(|command| {
				let timeout = self.factory.config.command_timeout(command.timeout);
				u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX)
			})
			.max()
			.unwrap_or(0);
		let config = VolumeClientConfig {
			open_timeout_ms: request_timeout_ms,
			quorum_timeout_ms: request_timeout_ms.max(longest_command_ms),
			payload_codec: self.factory.config.payload_codec.clone(),
			..Default::default()
		};
		let mut state = VolumeClientState::with_replica_set(config, key, tablet.replica_set)
			.map_err(|_| unavailable(UnavailableReason::Internal))?;
		// The maintainer creates a missing volume inside this open and grants its claim, so a
		// first write needs no separate create request.
		if create_if_absent {
			state.create_if_absent();
		}
		let lifecycle = self
			.factory
			.lifecycle
			.clone()
			.with_dispatch_guard(may_serve.clone());
		let make_transport = |guard: Arc<dyn Fn() -> bool + Send + Sync>| {
			let authority = may_serve.clone();
			Ok(Arc::new(UpsVolumeTransport::new(
				UpsLogClient::new(
					self.factory.pubsub.clone(),
					self.factory.log_capacities,
					Duration::from_secs(5),
					Arc::new(move || guard() && authority()),
				)?
				.with_lanes(self.factory.log_lanes.clone()),
				Arc::new(lifecycle),
			))
				as Arc<
					dyn depot_volume::volume_client_machine::executor::VolumeClientTransport,
				>)
		};
		let ready_timer = crate::metrics::KV_VOLUME_OPEN_PHASE_DURATION
			.with_label_values(&[command, "volume_ready"])
			.start_timer();
		// The opening's shared state and range locks; a speculative first command that created
		// the volume already holds its own, so the opening keeps them.
		let mut opening_shared: Arc<super::OpeningState> = Default::default();
		let mut opening_locked_ranges: Arc<Mutex<BTreeMap<Bytes, LockEntry>>> = Default::default();
		let initial_writes = speculative.is_some();
		let (volume, ready, created_run) = match speculative {
			// The open carries the first command's writes and waits for its reply, which says
			// whether it created the volume with them. Only then does the speculative result
			// stand: the volume was empty, so the command's reads of nothing were true.
			Some(mut run) => {
				let writes = std::mem::take(&mut run.writes);
				let (volume, opened) = self
					.factory
					.lifecycle
					.open_or_create_with(
						self.env.clone(),
						state,
						None,
						writes,
						None,
						make_transport,
					)
					.await
					.map_err(|_| unavailable(UnavailableReason::OpenFailed))?;
				let created = matches!(opened, Ok(Opened::Created(_)));
				run.journal.decide(created);
				// The command's commit is durable now. It resumes once the opened filesystem
				// below can take its later transactions. A command whose writes did not create
				// the volume stops at its failed commit.
				let created_run = match created {
					true => {
						opening_shared = run.shared.clone();
						opening_locked_ranges = run.locked_ranges.clone();
						Some(run)
					}
					false => None,
				};
				(volume, opened.map(|_| ()), created_run)
			}
			None => {
				let volume = self
					.factory
					.lifecycle
					.start_volume(self.env.clone(), state, None, make_transport)
					.map_err(|_| unavailable(UnavailableReason::OpenFailed))?;
				// A volume this open created runs its first command while its claim entry is in
				// flight, so the command's commit rides right behind the claim entry instead of
				// waiting for it. A failed claim entry fails that commit, and the caller retries.
				let handle = volume.handle();
				let behind_claim_entry =
					create_if_absent && handle.claim_entry_sent().await.unwrap_or(false);
				let ready = if behind_claim_entry {
					Ok(())
				} else {
					handle.ready().await
				};
				(volume, ready, None)
			}
		};
		ready_timer.observe_duration();
		let opening: Result<(Arc<Filesystem>, u64), KvCommandResponse> = async {
			if let Err(error) = ready {
				return Err(readiness_failure(error, initial_writes));
			}
			let journal = Arc::new(VolumeJournal::new(
				volume.handle(),
				may_serve,
				self.factory.pubsub.topology().clone(),
			));
			let cold = Arc::new(RemoteColdStore::new(
				Arc::new(self.factory.lifecycle.clone()),
				journal.clone(),
				self.env.clock(),
			));
			let filesystem_timer = crate::metrics::KV_VOLUME_OPEN_PHASE_DURATION
				.with_label_values(&[command, "filesystem_open"])
				.start_timer();
			// A volume that never committed a write skips its cold and log reads and formats in
			// its first commit, which also creates the control inode.
			let (filesystem, staged_ino) = Filesystem::open_staging_kv(
				journal,
				cold,
				self.env.clock(),
				self.factory.config.filesystem.clone(),
				self.factory.config.process_cache.clone(),
				b"kv",
			)
			.await
			.map_err(|error| {
				// A refused open ran no handler. Without a speculative first command it carried
				// no application writes either, so the edge may send the command again after the
				// hint, as when the maintainer sheds a read the full memtable budget would hold.
				let refused = matches!(
					error,
					FilesystemError::Overloaded
						| FilesystemError::Busy
						| FilesystemError::Lagging
						| FilesystemError::TooOld
						| FilesystemError::ColdReadRefused
						| FilesystemError::RecordMissing { .. }
				);
				if refused && !initial_writes {
					tracing::debug!(?error, "command volume filesystem open was refused");
					return filesystem_refusal(&error);
				}
				tracing::warn!(
					?error,
					"command volume filesystem failed to open; the caller retries"
				);
				unavailable(UnavailableReason::OpenFailed)
			})?;
			filesystem_timer.observe_duration();
			let inode_timer = crate::metrics::KV_VOLUME_OPEN_PHASE_DURATION
				.with_label_values(&[command, "inode"])
				.start_timer();
			let lookup = match staged_ino {
				Some(ino) => Ok(depot_fs_core::FsDentry {
					child: ino,
					kind: Kind::Kv,
				}),
				None => filesystem.lookup(depot_fs_core::ROOT_INO, b"kv").await,
			};
			let ino = match lookup {
				Ok(entry) if entry.kind == Kind::Kv => entry.child,
				Ok(_) => return Err(unavailable(UnavailableReason::Internal)),
				Err(FilesystemError::Filesystem(depot_fs_core::Error::NotFound)) => filesystem
					.create(
						depot_fs_core::ROOT_INO,
						b"kv",
						depot_fs_core::rules::new_inode(Kind::Kv, 0o600, 0, 0, 0),
					)
					.await
					.map_err(|_| unavailable(UnavailableReason::Internal))?,
				// A refused cold read ran nothing, so the edge may send the command again, as when
				// the filesystem open above is refused.
				Err(error @ FilesystemError::ColdReadRefused) if !initial_writes => {
					return Err(filesystem_refusal(&error));
				}
				Err(_) => return Err(unavailable(UnavailableReason::Internal)),
			};
			inode_timer.observe_duration();
			Ok((filesystem, ino))
		}
		.await;
		// The speculative command that created the volume resumes after its creating commit,
		// with its handle moved to the opened filesystem first, so its later transactions and
		// its effects' writes land on the volume. It runs to its end even when the opening
		// failed, because its commit is durable and its effects must still be dispatched.
		let speculated = match created_run {
			Some(run) => {
				let moved = opening
					.as_ref()
					.ok()
					.map(|(filesystem, ino)| (filesystem.clone(), *ino));
				run.writer.resolve(moved);
				let result = run.handler.await;
				Some(super::speculation::Speculated {
					operation_id: run.operation_id,
					result,
					caller_timed_out: run.commits.expired(),
				})
			}
			None => None,
		};
		let (filesystem, ino) = opening?;
		Ok(OpenVolume {
			filesystem,
			ino,
			volume,
			locked_ranges: opening_locked_ranges,
			shared: opening_shared,
			speculated: parking_lot::Mutex::new(speculated),
		})
	}

	/// Runs `speculation` against an empty in-memory volume up to its first commit, which
	/// then waits for [`depot_client::filesystem::capture::CaptureJournal::decide`]. Returns `None` when the
	/// command finished or failed without committing, so it runs as usual after the open.
	async fn speculate(
		&self,
		key: &depot_types::VolumeKey,
		speculation: super::speculation::Speculation,
		may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Option<super::speculation::SpeculativeRun> {
		let command = self.factory.registry.commands.get(&speculation.tag)?;
		let journal = Arc::new(depot_client::filesystem::capture::CaptureJournal::new(
			depot_types::VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: [0; 16],
			}),
			self.factory.pubsub.topology().clone(),
		));
		let (filesystem, staged) = Filesystem::open_staging_kv(
			journal.clone(),
			Arc::new(depot_client::filesystem::capture::NoColdStore),
			self.env.clock(),
			self.factory.config.filesystem.clone(),
			None,
			b"kv",
		)
		.await
		.ok()?;
		let shared: Arc<super::OpeningState> = Default::default();
		let locked_ranges: Arc<Mutex<BTreeMap<Bytes, LockEntry>>> = Default::default();
		let writer = super::speculation::SpeculationGuard::new();
		let kv = KvHandle {
			alarm_owner: self.alarm_owner.get().cloned(),
			filesystem,
			ino: staged?,
			may_serve,
			operation_id: speculation.operation_id,
			volume_key: key.clone(),
			locked_ranges: locked_ranges.clone(),
			shared: shared.clone(),
			commits: speculation.commits.clone(),
			speculative: Some(writer.writer()),
			versions: self.factory.pubsub.topology().clone(),
		};
		let mut handler = command
			.handler
			.run_with_writer(kv, speculation.body.to_vec());
		let committing = rivet_env::select_biased!("a finished command needs no speculative open",
			_ = &mut handler => false,
			() = journal.commit_requested() => true,
		);
		let writes = journal.initial_writes();
		if !committing || writes.is_empty() {
			journal.decide(false);
			return None;
		}
		Some(super::speculation::SpeculativeRun {
			operation_id: speculation.operation_id,
			shared,
			locked_ranges,
			writer,
			commits: speculation.commits.clone(),
			writes,
			journal,
			handler,
		})
	}

	fn evicted(&self, cache: &mut Cache, now: u64, idle: u64) {
		crate::metrics::KV_VOLUME_EVICTIONS.inc();
		crate::metrics::KV_VOLUME_EVICTED_IDLE.observe(idle as f64 / 1000.0);
		while cache
			.recent_short_evictions
			.front()
			.is_some_and(|time| now.saturating_sub(*time) >= 60_000)
		{
			cache.recent_short_evictions.pop_front();
		}
		if idle < self.factory.config.evicted_idle_warn_ms {
			cache.recent_short_evictions.push_back(now);
		}
		let churning = !cache.recent_short_evictions.is_empty();
		if churning && !cache.churn_warning {
			tracing::warn!(
				cap = self.factory.config.max_open_volumes,
				evictions_last_minute = cache.recent_short_evictions.len(),
				"KV volume cache is churning; increase its capacity"
			);
		}
		cache.churn_warning = churning;
	}

	fn sweep_interval_ms(&self) -> u64 {
		self.factory.config.sweep_interval_ms
	}

	fn command_capacity(&self) -> usize {
		Self::command_capacity_of(&self.factory)
	}

	fn command_capacity_of(factory: &Factory) -> usize {
		factory
			.registry
			.commands
			.values()
			.map(|command| {
				command
					.concurrency
					.min(factory.config.max_inflight_commands)
			})
			.sum()
	}

	fn observe(&self, cache: &mut Cache) {
		for (index, (kind, used, cap)) in [
			(
				"host_volumes",
				cache.volumes.len(),
				self.factory.config.max_open_volumes,
			),
			("commands", cache.inflight, self.command_capacity()),
			(
				"bytes",
				cache.inflight_bytes,
				self.factory.config.max_inflight_command_bytes,
			),
			(
				"eviction_closes",
				cache.evicting,
				self.factory.config.max_eviction_closes,
			),
		]
		.into_iter()
		.enumerate()
		{
			// Warn on reaching four fifths of the cap and re-arm only below half of it, so usage
			// held at the threshold logs once per episode instead of once per command.
			let near = used.saturating_mul(5) >= cap.saturating_mul(4);
			let recovered = used.saturating_mul(2) < cap;
			if near && !cache.near_cap[index] {
				tracing::warn!(
					kind,
					used,
					cap,
					"KV command capacity is near its cap; commands past it are refused and retried"
				);
				cache.near_cap[index] = true;
			} else if recovered && cache.near_cap[index] {
				tracing::info!(kind, used, cap, "KV command capacity recovered");
				cache.near_cap[index] = false;
			}
		}
	}
}

struct Admission {
	tag: CommandTag,
	ready: watch::Receiver<bool>,
	active: Arc<AtomicBool>,
	cancel: CancellationToken,
	host: Arc<VolumeHost>,
	key: depot_types::VolumeKey,
	opened: Option<Arc<OnceCell<Result<OpenVolume, KvCommandResponse>>>>,
	bytes: usize,
	/// Whether this command holds a volume slot. A writerless read holds only command capacity.
	slotted: bool,
	/// Counts this command and its bytes on `KV_USAGE` while it is admitted.
	_usage_metrics: [IntGaugeGuard; 2],
}

impl Admission {
	fn usage_metrics(bytes: usize) -> [IntGaugeGuard; 2] {
		[
			crate::metrics::KV_USAGE
				.with_label_values(&["commands"])
				.inc_guard(),
			crate::metrics::KV_USAGE
				.with_label_values(&["bytes"])
				.add_guard(bytes as i64),
		]
	}
}

impl Drop for Admission {
	fn drop(&mut self) {
		// Release the extra volume reference before making the slot eligible for eviction.
		self.opened.take();
		let mut cache = self.host.lock_cache();
		cache.inflight -= 1;
		cache.inflight_bytes -= self.bytes;
		if let Some(count) = cache.by_command.get_mut(&self.tag) {
			*count -= 1;
			if *count == 0 {
				cache.by_command.remove(&self.tag);
			}
		}
		let now = self.host.env.clock().observe().map_or(0, |now| now.boot_ms);
		if self.slotted {
			cache.command_finished(&self.key, now, self.host.sweep_interval_ms());
		}
		self.host.observe(&mut cache);
		self.host.publish(&cache);
	}
}

impl Drop for VolumeHost {
	fn drop(&mut self) {
		self.active.store(false, Ordering::Release);
		self.tasks.cancel();
		if let (Some(budget), Some(id)) = (&self.factory.config.worker_volumes, self.budget_id) {
			budget.deregister(id);
		}
	}
}

/// Why the host closed a volume slot, for the close metric.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseReason {
	/// The slot stayed idle for its idle close.
	Idle,
	/// Deletion or revocation deactivated the slot.
	Deactivated,
	/// The open failed; the failed outcome is not worth caching.
	FailedOpen,
	/// Admission found the slot fenced, deactivated or missing its volume.
	Discarded,
	/// Admission at capacity closed the slot to make room.
	Evicted,
	/// An operator closed the idle slot before its idle close.
	Operator,
}

impl CloseReason {
	fn label(self) -> &'static str {
		match self {
			CloseReason::Idle => "idle",
			CloseReason::Deactivated => "deactivated",
			CloseReason::FailedOpen => "failed_open",
			CloseReason::Discarded => "discarded",
			CloseReason::Evicted => "evicted",
			CloseReason::Operator => "operator",
		}
	}
}

/// The registered name of a command, a bounded metric label.
fn command_label(tag: CommandTag) -> &'static str {
	rivet_keys::keys::tags::depot_command::str_from_key(tag.0).unwrap_or("unknown")
}

/// A bounded result label for a finished open, so open failures can be told apart.
fn open_result_label(result: &Result<OpenVolume, KvCommandResponse>) -> &'static str {
	match result {
		Ok(_) => "ok",
		Err(response) => refusal_label(response),
	}
}

/// A bounded metric label for a refused or failed command response.
pub(crate) fn refusal_label(response: &KvCommandResponse) -> &'static str {
	match response {
		KvCommandResponse::Overloaded(overloaded) => {
			depot_client::overload_reason_label(overloaded.reason)
		}
		KvCommandResponse::Unavailable(unavailable) => {
			depot_client::unavailable_reason_label(unavailable.reason)
		}
		KvCommandResponse::AuthorityMoved(_) => "authority_moved",
		KvCommandResponse::Fenced => "fenced",
		KvCommandResponse::Corrupt => "corrupt",
		KvCommandResponse::Conflict => "conflict",
		KvCommandResponse::VolumeNotFound => "volume_not_found",
		KvCommandResponse::VolumeDeleted => "volume_deleted",
		KvCommandResponse::NotMaintainer => "not_maintainer",
		KvCommandResponse::CommandTooLarge => "command_too_large",
		KvCommandResponse::UnknownCommand => "unknown_command",
		KvCommandResponse::CommandResult(_) | KvCommandResponse::ApplicationError(_) => "error",
	}
}

/// The typed refusal for a filesystem failure under a command. Overload and short volume
/// transitions are retryable refusals; a fence or corruption is final, so a caller never retries
/// it into a deadline.
fn filesystem_refusal(error: &FilesystemError) -> KvCommandResponse {
	match error {
		FilesystemError::Overloaded | FilesystemError::Busy => {
			overloaded(OverloadReason::FilesystemOverloaded)
		}
		// A refused cold read did no work and reads are idempotent. Like the transitions above,
		// a handler that already committed an earlier transaction relies on the handler
		// idempotency contract when the edge sends the command again.
		FilesystemError::Lagging | FilesystemError::TooOld | FilesystemError::ColdReadRefused => {
			overloaded(OverloadReason::VolumeTransition)
		}
		// The write was refused before it was built because the volume's writes are ahead of
		// its tablet's apply, so the edge may send the command again after the hint.
		FilesystemError::ApplyBacklog { retry_after_ms } => {
			overloaded_after(OverloadReason::FilesystemOverloaded, *retry_after_ms)
		}
		FilesystemError::Fenced => KvCommandResponse::Fenced,
		// A volume whose stored record the store lost serves nothing until it is deleted, so
		// the caller must not retry it into a deadline either.
		FilesystemError::Corrupt | FilesystemError::RecordMissing { .. } => {
			KvCommandResponse::Corrupt
		}
		FilesystemError::Conflict => KvCommandResponse::Conflict,
		FilesystemError::TooLarge => KvCommandResponse::CommandTooLarge,
		FilesystemError::DeadlineExceeded => unavailable(UnavailableReason::TimedOut),
		// The volume client refused this write before dispatching it, so the write did not run
		// and the edge may send the command again. A handler that already committed an earlier
		// transaction relies on the handler idempotency contract, as above.
		FilesystemError::Log(error) if error.disposition() == ErrorDisposition::NotExecuted => {
			match error.retry_after_ms() {
				Some(retry_after_ms) => {
					overloaded_after(OverloadReason::FilesystemOverloaded, retry_after_ms)
				}
				None => overloaded(OverloadReason::FilesystemOverloaded),
			}
		}
		FilesystemError::Unavailable
		| FilesystemError::BaseUnavailable
		| FilesystemError::Log(_)
		| FilesystemError::Filesystem(_) => unavailable(UnavailableReason::StorageFailed),
	}
}

impl Config {
	/// The registered handler timeout, raised to the configured floor when one is set. This
	/// matches the client's deadline so a widened retry budget is not cut short by the handler.
	pub fn command_timeout(&self, registered: Duration) -> Duration {
		match self.command_timeout_floor {
			Some(floor) => registered.max(floor),
			None => registered,
		}
	}

	pub fn from_depot(config: &rivet_config::config::DepotConfig) -> Self {
		let defaults = Self::default();
		let overrides = &config.kv_commands;
		Self {
			request_capacity: overrides.request_capacity(),
			// A host's own cap defaults to its worker's, so the worker budget refuses first.
			max_open_volumes: overrides
				.max_open_volumes
				.unwrap_or_else(|| OpenVolumeBudget::capacity_of(overrides)),
			max_inflight_commands: overrides
				.max_inflight_commands
				.unwrap_or(defaults.max_inflight_commands),
			max_command_bytes: overrides
				.max_command_bytes
				.unwrap_or(defaults.max_command_bytes),
			max_inflight_command_bytes: overrides.max_inflight_command_bytes(),
			volume_idle_close_ms: overrides
				.volume_idle_close_ms
				.unwrap_or(defaults.volume_idle_close_ms),
			evicted_idle_warn_ms: overrides
				.evicted_idle_warn_ms
				.unwrap_or(defaults.evicted_idle_warn_ms),
			max_eviction_closes: overrides.max_eviction_closes(),
			filesystem: config.filesystem.clone(),
			process_cache: None,
			worker_volumes: None,
			command_timeout_floor: config.command_timeout_floor(),
			sweep_interval_ms: defaults.sweep_interval_ms,
			manifest_refresh: None,
			log_inflight_per_worker: config.log_client_inflight_per_worker(),
			payload_codec: match config.volume_client.payload_codec {
				Some(rivet_config::config::VolumePayloadCodec::None) => {
					rivet_data::generated::depot_log_shard_v1::PayloadCodec::None
				}
				Some(rivet_config::config::VolumePayloadCodec::Lz4) | None => {
					rivet_data::generated::depot_log_shard_v1::PayloadCodec::Lz4
				}
			},
		}
	}
}
