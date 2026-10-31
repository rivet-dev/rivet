//! Exclusive filesystem owner. The volume log owns replication and retry state;
//! this facade serializes validation/admission and retains immutable read versions.

use super::{
	Config, Error, Result,
	cache::{Budget, CacheBudget, CacheShare, Charge, Cold, Layer, RowCache, View},
	transaction::{ConflictWindow, Pin, Queue},
	transport::{ColdSnapshot, ColdStore, Journal, LogStatus, OpenPrefetch, OpenedSnapshot},
};
use depot_fs_core::{
	store::{Row, RowKey, RowRange, RowStore},
	*,
};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::runner::LogHistory;
use futures_util::{StreamExt, TryStreamExt};
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use rivet_env::Clock;
use std::{
	collections::BTreeMap,
	sync::{
		Arc, Weak,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
};
use tokio::sync::Mutex;

/// Batches of one multi-range read in flight at once. Each holds up to half of
/// `max_operation_bytes`, so a read never holds more than a few times that of the memory
/// budget while still fetching a large opening read in one round.
const READ_BATCHES_IN_FLIGHT: usize = 4;

/// A no-op captures its sequence under the write gate; a mutation carries durable evidence.
enum MutationOutcome {
	Unchanged(u64),
	Committed(depot_volume::volume_client_machine::types::WriteOutcome),
}



pub(super) struct State {
	/// The epoch of the volume client this filesystem opened on, which every entry it admits
	/// carries. The journal never changes client under the filesystem.
	pub client_epoch: u64,
	pub cold: Arc<Cold>,
	/// Until this time a memory-driven refresh waits after one moved the snapshot: a second
	/// below half the budget and 100 ms above it, so a busy writer does not move it per
	/// request.
	pub memory_release_after_boot_ms: u64,
	/// The overlay above the snapshot: one layer per entry the snapshot does not include, in
	/// ascending sequence, each with the time its notes expire. Only [`Filesystem::push_layer`]
	/// adds a written entry's layer, so every one carries a note timer.
	pub layers: Vec<Note>,
	/// The epoch of each entry above the snapshot that another writer wrote, kept until a
	/// snapshot passes it. An entry above the snapshot with no entry here is this filesystem's
	/// own, at its client epoch: the note timer drops those positions with their notes.
	pub positions: BTreeMap<u64, (u64, Charge)>,
	pub last_seq: u64,
	pub resolver: txn::Resolver,
	pub transactions: BTreeMap<Vec<u8>, super::txn::Pending>,
	pub next_transaction: u64,
	pub conflicts: ConflictWindow,
	pub pins: Vec<Weak<Pin>>,
	pub handles: BTreeMap<u64, usize>,
	pub locks: BTreeMap<u64, Vec<FileLock>>,
	/// Formatting ops of a volume that has never committed a write, waiting for the first
	/// commit to carry them. Their rows sit in a layer at sequence 0, so every view sees them.
	pub staged_format: Option<StagedFormat>,
	/// The payload of an admission whose outcome this opening did not observe, such as one
	/// that passed its deadline. The volume client may still admit it, as the next entry,
	/// until a later request's barrier orders behind it; that request reads it back and
	/// installs it, or finds it never landed.
	pub unobserved_admission: Option<vbare::Bytes>,
	/// This opening's admitted bytes the maintainer has not reported applied.
	pub backlog: super::backlog::UnappliedBacklog,
}

/// One entry's layer above the snapshot and when its notes, "this page is at this sequence",
/// expire. After that the filesystem drops the layer, keeps its rows only as cache, and moves
/// its snapshot's floor past the entry, so a miss reads the entry's rows from the maintainer.
/// An entry the maintainer has not reported applied keeps its layer as an index instead, so a
/// miss reads it from the log and never waits for a lagging apply.
pub(super) struct Note {
	pub layer: Arc<Layer>,
	pub deadline: NoteDeadline,
}

/// When a layer's notes stop holding it above the snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteDeadline {
	/// The note timer considers the layer once this boot time passes.
	At(u64),
	/// The notes expired while the entry was above the newest position the maintainer had
	/// reported applied. The layer holds only its index, and its rows are read from the log.
	/// The next refresh that finds the entry applied releases it with a floor move; no timer
	/// runs for it meanwhile.
	UntilApplied,
}

impl Note {
	/// A staged format layer at sequence zero never expires: its rows exist nowhere else until
	/// the first commit carries them, and a snapshot install drops it.
	fn staged(layer: Arc<Layer>) -> Self {
		Self {
			layer,
			deadline: NoteDeadline::At(u64::MAX),
		}
	}

	/// Whether the note timer still has to consider this layer.
	fn timed(&self) -> bool {
		self.layer.seq > 0 && matches!(self.deadline, NoteDeadline::At(_))
	}
}

/// What the note timer found when it ran, for the owner that schedules it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteExpiry {
	/// No note is pending; the filesystem arms nothing until a write adds one, which runs the
	/// owner's [`Filesystem::on_notes_armed`] signal.
	Idle,
	/// The earliest remaining note is due at this time.
	Due(u64),
	/// An operation holds the filesystem; the owner checks again after its sweep interval.
	Busy,
}

/// The format of an empty volume and any files staged with it, written by its first commit
/// instead of at open. Each entry is valid only at its sequence, because an inode it creates
/// is numbered by that sequence.
pub(super) struct StagedFormat {
	/// Consecutive entries from the next log sequence. The first holds the root.
	pub entries: Vec<StagedEntry>,
}

#[derive(Clone)]
pub(super) struct StagedEntry {
	pub seq: u64,
	pub ops: Vec<FsOp>,
}

/// One file staged into an empty volume's first commit, created inside the previous file of
/// the chain or, for the first, inside the root.
#[derive(Clone, Copy, Debug)]
pub struct StagedFile<'a> {
	pub name: &'a [u8],
	pub kind: FsKind,
	pub mode: u32,
}

/// The cold view of a volume that has never written cold rows and has no fork base: every
/// row is absent. The first floor move replaces it with the store's claim snapshot.
struct EmptyColdSnapshot {
	position: rivet_data::generated::shared_depot_types_v1::VolumeLogPosition,
}

impl ColdSnapshot for EmptyColdSnapshot {
	fn position(&self) -> &rivet_data::generated::shared_depot_types_v1::VolumeLogPosition {
		&self.position
	}

	fn get(
		&self,
		_key: RowKey,
		_through: u64,
		_deadline_boot_ms: u64,
	) -> futures_util::future::BoxFuture<'_, Result<Option<Row>>> {
		Box::pin(async { Ok(None) })
	}

	fn scan(
		&self,
		_range: RowRange,
		_after: Option<RowKey>,
		_limit: usize,
		_through: u64,
		_deadline_boot_ms: u64,
	) -> futures_util::future::BoxFuture<'_, Result<Vec<(RowKey, Row)>>> {
		Box::pin(async { Ok(Vec::new()) })
	}
}

/// Opens the snapshot an opening replays the log above. A latest open serves the maintainer's
/// settled position, the highest applied position with no transaction half applied, which is
/// usually far above cold, so the opening replays only what the maintainer has not applied and
/// never a transaction whose start it cannot see. The position is used only when it is one of
/// this volume's committed entries; otherwise, or when the maintainer is still behind cold, the
/// opening reads at exactly cold and replays everything above it.
async fn open_replay_snapshot(
	journal: &dyn Journal,
	cold_store: &dyn ColdStore,
	clock: &dyn Clock,
	key: &VolumeLogKey,
	history: &LogHistory,
	prefetch: &OpenPrefetch,
	deadline: u64,
) -> Result<OpenedSnapshot> {
	let latest = rivet_env::timeout_at(
		clock,
		deadline,
		cold_store.open_latest_prefetching(
			key.clone(),
			history.cold.clone(),
			prefetch.clone(),
			deadline,
		),
	)
	.await
	.map_err(|_| Error::DeadlineExceeded)?;
	match latest {
		Ok(opened) if opened.snapshot.position() == &history.cold => return Ok(opened),
		Ok(opened)
			if is_committed_entry(
				journal,
				key,
				history,
				opened.snapshot.position(),
				deadline,
				clock,
			)
			.await? =>
		{
			return Ok(opened);
		}
		Ok(_) | Err(Error::Lagging) => {}
		Err(error) => return Err(error),
	}
	rivet_env::timeout_at(
		clock,
		deadline,
		cold_store.open_prefetching(
			key.clone(),
			history.cold.clone(),
			prefetch.clone(),
			deadline,
		),
	)
	.await
	.map_err(|_| Error::DeadlineExceeded)?
}

/// Whether `position` names a committed entry of this volume's log at its own client epoch,
/// above cold. An applied position from the maintainer that does not is evidence of another
/// writer, so it fences.
async fn is_committed_entry(
	journal: &dyn Journal,
	key: &VolumeLogKey,
	history: &LogHistory,
	position: &VolumeLogPosition,
	deadline: u64,
	clock: &dyn Clock,
) -> Result<bool> {
	if position.volume_seq <= history.cold.volume_seq || position.volume_seq > history.committed {
		return Ok(false);
	}
	let epoch = match history
		.entries
		.iter()
		.find(|entry| entry.volume_seq == position.volume_seq)
	{
		Some(entry) => entry.client_epoch,
		None => {
			let page = rivet_env::timeout_at(
				clock,
				deadline,
				journal.read(
					position.volume_seq,
					journal.status()?.max_payload_bytes + 128,
				),
			)
			.await
			.map_err(|_| Error::DeadlineExceeded)??;
			match page
				.entries
				.into_iter()
				.find(|entry| entry.volume_seq == position.volume_seq)
			{
				Some(entry) => entry.client_epoch,
				None => return Ok(false),
			}
		}
	};
	if epoch != position.client_epoch {
		crate::fences::record(
			crate::fences::FenceReason::OpenColdMismatch,
			Some(key),
			journal.client_epoch(),
			&AtomicBool::new(false),
		);
		journal.fence();
		return Err(Error::Fenced);
	}
	Ok(true)
}

/// A non-blocking POSIX advisory lock owned by an open handle identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileLock {
	pub owner: u64,
	pub start: u64,
	pub end: Option<u64>,
	pub exclusive: bool,
}

impl FileLock {
	fn conflicts(&self, other: &Self) -> bool {
		self.owner != other.owner
			&& (self.exclusive || other.exclusive)
			&& self.end.is_none_or(|end| other.start < end)
			&& other.end.is_none_or(|end| self.start < end)
	}
}

pub struct Filesystem {
	pub(super) journal: Arc<dyn Journal>,
	pub(super) cold_store: Arc<dyn ColdStore>,
	pub(super) write_gate: Mutex<()>,
	pub(super) clock: Arc<dyn Clock>,
	pub(super) config: Config,
	pub(super) budget: Arc<Budget>,
	pub(super) rows: Arc<RowCache>,
	pub(super) cache_share: Arc<CacheShare>,
	pub(super) state: Mutex<State>,
	pub(super) queue: parking_lot::Mutex<Queue>,
	pub(super) transaction_wake: tokio::sync::Notify,
	pub(super) queue_wake: tokio::sync::Notify,
	pub(super) key: VolumeLogKey,
	pub(super) fenced: AtomicBool,
	/// The highest sequence whose commit wait ended undecided while this opening kept serving,
	/// or 0. Until the volume client reports it committed, every request that refreshes the
	/// view first waits for it within its own deadline, so nothing builds on or serves an
	/// entry that a newer claim could still discard.
	pub(super) undecided_through: AtomicU64,
	/// Whether this opening logged its first fence cause.
	fence_logged: AtomicBool,
	/// Runs when a write adds the first pending note, so the owner schedules the note timer.
	notes_armed: parking_lot::Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

/// Cancellation after dispatch is an uncertain result, not rollback. The log
/// retains the exact entry/identity for its existing recovery protocol.
pub(super) struct Attempt<'a> {
	fs: &'a Filesystem,
	armed: bool,
}
impl<'a> Attempt<'a> {
	pub(crate) fn new(fs: &'a Filesystem) -> Self {
		Self { fs, armed: true }
	}
	pub(crate) fn finish(mut self) {
		self.armed = false;
	}
}
impl Drop for Attempt<'_> {
	fn drop(&mut self) {
		if self.armed {
			self.fs
				.fence_with(crate::fences::FenceReason::AttemptAbandoned);
		}
	}
}

impl Filesystem {
	pub(crate) fn volume_key(&self) -> &VolumeLogKey {
		&self.key
	}

	/// The volume incarnation this filesystem owns, for deleting exactly it.
	pub fn volume_log_key(&self) -> &VolumeLogKey {
		&self.key
	}
	/// Capture once after the volume client has announced the confirmed position.
	/// An uncertain fork retry keeps this value rather than rebasing onto new writes.
	/// The position names the epoch of the entry at `seq`, which after a settled open may be an
	/// earlier writer's, or the cold position when nothing above it is held.
	pub(crate) async fn fork_minimum(&self, seq: u64) -> Result<FsForkMinimum> {
		let status = self.status()?;
		if seq > status.committed {
			return Err(self.fenced(crate::fences::FenceReason::ForkMinimumUncommitted));
		}
		let state = self.state.lock().await;
		let cold = state.cold.snapshot.position();
		let position = if cold.volume_seq >= seq {
			cold.clone()
		} else {
			rivet_data::generated::shared_depot_types_v1::VolumeLogPosition {
				volume_seq: seq,
				client_epoch: entry_epoch(&state, seq),
			}
		};
		Ok(FsForkMinimum {
			incarnation: self.key.0.volume_incarnation,
			position,
		})
	}

	pub(crate) async fn checkpoint_entry(
		&self,
		name: Vec<u8>,
	) -> Result<rivet_data::generated::depot_client_protocol_v1::CheckpointPosition> {
		if !depot_fs_core::checkpoints::valid_name(&name) {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		self.administrative_entry(|now| {
			FsOp::FsCheckpoint(FsCheckpoint {
				name: name.clone().into(),
				created_at: now,
			})
		})
		.await
	}

	/// Appends `FsCheckpointRelease` as its own entry and returns its committed position, which
	/// the maintainer confirms once its apply reaches it.
	pub(crate) async fn checkpoint_release_entry(
		&self,
		incarnation: [u8; 16],
		name: Vec<u8>,
	) -> Result<rivet_data::generated::depot_client_protocol_v1::CheckpointPosition> {
		self.administrative_entry(|_| {
			FsOp::FsCheckpointRelease(FsCheckpointRelease {
				name: name.clone().into(),
				incarnation,
			})
		})
		.await
	}

	/// Appends `FsRetentionSet` as its own entry and returns its committed position.
	pub(crate) async fn retention_entry(
		&self,
		policy: Option<FsRetentionPolicy>,
	) -> Result<rivet_data::generated::depot_client_protocol_v1::CheckpointPosition> {
		self.administrative_entry(|_| {
			FsOp::FsRetentionSet(FsRetentionSet {
				policy: policy.clone(),
			})
		})
		.await
	}

	/// Appends the one op `build` makes from the write time as its own entry, and returns the
	/// position the maintainer confirms it at.
	async fn administrative_entry(
		&self,
		build: impl Fn(i64) -> FsOp,
	) -> Result<rivet_data::generated::depot_client_protocol_v1::CheckpointPosition> {
		let MutationOutcome::Committed(committed) =
			self.mutate_entry(|_, _, now| Ok(vec![build(now)])).await?
		else {
			return Err(corrupt!());
		};
		Ok(
			rivet_data::generated::depot_client_protocol_v1::CheckpointPosition {
				volume_incarnation: self.key.0.volume_incarnation,
				position: committed.position,
			},
		)
	}

	/// Replays one claimed committed history, then creates the root if necessary
	/// and reclaims orphaned inodes. Starting the underlying journal is explicit.
	pub async fn open(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
	) -> Result<Arc<Self>> {
		Self::open_with_cache_budget(journal, cold_store, clock, config, None).await
	}

	/// Like [`Filesystem::open`], with caches that also reserve against a process-wide budget.
	pub async fn open_with_cache_budget(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
	) -> Result<Arc<Self>> {
		let (fs, _) = Self::open_inner(
			journal,
			cold_store,
			clock,
			config,
			process_cache,
			None,
			OpenPrefetch::default(),
		)
		.await?;
		Ok(fs)
	}

	/// Like [`Filesystem::open_with_cache_budget`], but a volume that has never committed a
	/// write reads neither cold rows nor the log: its root is staged into its first commit,
	/// and [`Filesystem::stage_files`] can stage files beside it. A volume never written
	/// stays empty, so the next open stages again. `prefetch` names what the opening reads
	/// first, which the cold snapshot's open then answers in its own reply.
	pub async fn open_staging_empty(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
		prefetch: OpenPrefetch,
	) -> Result<Arc<Self>> {
		let (fs, _) = Self::open_inner(
			journal,
			cold_store,
			clock,
			config,
			process_cache,
			Some(&[]),
			prefetch,
		)
		.await?;
		Ok(fs)
	}

	/// Like [`Filesystem::open_with_cache_budget`] for a volume that holds one KV inode named
	/// `kv_name` under the root. When the volume has never committed a write, the open stages
	/// the root and the KV inode into the first commit and returns the inode. Otherwise it
	/// returns `None`, and the caller looks the inode up. A handler that never commits leaves
	/// the volume empty, so the next open stages again.
	pub async fn open_staging_kv(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
		kv_name: &[u8],
	) -> Result<(Arc<Self>, Option<u64>)> {
		let kv = StagedFile {
			name: kv_name,
			kind: FsKind::Kv,
			mode: 0o600,
		};
		// A volume that holds the KV inode already answers its lookup with the open.
		let prefetch = OpenPrefetch {
			path: vec![kv_name.to_vec()],
			xattrs: Vec::new(),
			kv: Vec::new(),
		};
		let (fs, staged) = Self::open_inner(
			journal,
			cold_store,
			clock,
			config,
			process_cache,
			Some(&[kv]),
			prefetch,
		)
		.await?;
		Ok((fs, staged.and_then(|inos| inos.first().copied())))
	}

	async fn open_inner(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
		staging: Option<&[StagedFile<'_>]>,
		prefetch: OpenPrefetch,
	) -> Result<(Arc<Self>, Option<Vec<u64>>)> {
		config
			.validate()
			.map_err(|_| Error::Filesystem(depot_fs_core::Error::Invalid))?;
		let now = clock.observe().map_err(|_| Error::Unavailable)?.boot_ms;
		let deadline = now.saturating_add(config.request_timeout_ms);

		// The cold store can move past the position an open is reading at, between learning the
		// history and opening its snapshot or reading the log above it. The history is then
		// gone from where the open looked, not from the volume, so the open learns the newer
		// cold position and starts again from a fresh history, within its deadline. Nothing was
		// written, so the journal stays open.
		let mut restarted_without_evidence = false;
		loop {
			// A volume client still reopening has no status yet; recovery inside the attempt
			// waits for it, so a missing status only means no cold position to compare.
			let cold_before = journal.status().map_or(0, |status| status.cold.volume_seq);
			let attempt = Self::open_once(
				journal.clone(),
				cold_store.clone(),
				clock.clone(),
				config.clone(),
				process_cache.clone(),
				staging,
				&prefetch,
				deadline,
			)
			.await;
			match attempt {
				Err(Error::TooOld) if clock.observe().is_ok_and(|now| now.boot_ms < deadline) => {
					crate::metrics::FS_OPEN_RESTARTS.inc();
					tracing::debug!("cold moved past a filesystem open; starting it again");
					// The journal's cold position is what the history starts from, so it must
					// learn the newer one from the replicas first, or the next attempt reads
					// the same trimmed history again. The failed attempt's snapshot is unused.
					rivet_env::timeout_at(clock.as_ref(), deadline, journal.confirm())
						.await
						.map_err(|_| Error::DeadlineExceeded)??;
					// Start again on evidence that cold moved. A moved maintainer can also reject
					// a read while cold stays put, so one restart is allowed without that
					// evidence. A further `TooOld` the replicas do not explain is returned instead
					// of retried in a loop.
					if journal.status()?.cold.volume_seq <= cold_before {
						if restarted_without_evidence {
							return Err(Error::TooOld);
						}
						restarted_without_evidence = true;
					}
				}
				result => return result,
			}
		}
	}

	/// One attempt of [`Self::open_inner`], from reading the history through replay and the
	/// root, bounded by `deadline`. Fails `TooOld` without fencing the journal when the cold
	/// store moved past the history it read.
	#[allow(
		clippy::too_many_arguments,
		reason = "one open's injected dependencies"
	)]
	async fn open_once(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
		staging: Option<&[StagedFile<'_>]>,
		prefetch: &OpenPrefetch,
		deadline: u64,
	) -> Result<(Arc<Self>, Option<Vec<u64>>)> {
		let phase = |name| {
			crate::metrics::FS_OPEN_PHASE_DURATION
				.with_label_values(&[name])
				.start_timer()
		};
		let (fs, status, committed, staged) = Self::read_open_view(
			journal,
			cold_store,
			clock,
			config,
			process_cache,
			staging.is_some(),
			prefetch,
			deadline,
		)
		.await?;
		let staging = staging.filter(|_| staged);
		let attempt = Attempt::new(&fs);
		if let Some(files) = staging {
			let timer = phase("stage_format");
			let inos = {
				let mut state = fs.state.lock().await;
				// A journal whose first entry is still to come, such as a speculative one, has no
				// claim entry to account for.
				if committed > 0 {
					fs.account_claim_entry(&mut state, &status)?;
				}
				fs.stage_format(&mut state, &status, files, deadline)
					.await?
			};
			drop(timer);
			attempt.finish();
			return Ok((fs, Some(inos)));
		}
		// The root and the first orphan page are independent reads. On a cold volume each is a
		// chain of object-store round trips, so they run together. The format is read only once
		// the root exists, because reading it on a volume without one fails as corrupt.
		let timer = phase("root");
		let (root, first_orphans) = futures_util::future::join(
			fs.getattr(ROOT_INO),
			fs.read_with(|v| v.scan(&RowRange::Orphans, None, 128, None)),
		)
		.await;
		match root {
			Ok(_) => {}
			Err(Error::Filesystem(depot_fs_core::Error::NotFound)) => {
				fs.mutate(|_, _, now| Ok(vec![rules::root(0o755, 0, 0, now)]))
					.await?;
			}
			Err(error) => return Err(error),
		}
		fs.read_with(depot_fs_core::store::format).await?;
		drop(timer);
		let timer = phase("orphans");
		let mut first_orphans = Some(first_orphans?);
		loop {
			let orphans = match first_orphans.take() {
				Some(orphans) => orphans,
				None => {
					fs.read_with(|v| v.scan(&RowRange::Orphans, None, 128, None))
						.await?
				}
			};
			if orphans.is_empty() {
				break;
			}
			for (key, _) in orphans {
				let RowKey::Orphan(ino) = key else {
					return Err(corrupt!());
				};
				fs.mutate(|_, _, _| Ok(vec![FsOp::FsReclaim(FsReclaim { ino })]))
					.await?;
			}
		}
		drop(timer);
		attempt.finish();
		Ok((fs, None))
	}

	/// Read-only recovery of one snapshot. The caller may discard TooOld evidence;
	/// no formatting, orphan reclamation, journal reopen or append occurs here.
	#[allow(
		clippy::too_many_arguments,
		reason = "one open's injected dependencies"
	)]
	async fn read_open_view(
		journal: Arc<dyn Journal>,
		cold_store: Arc<dyn ColdStore>,
		clock: Arc<dyn Clock>,
		config: Config,
		process_cache: Option<Arc<CacheBudget>>,
		staging: bool,
		prefetch: &OpenPrefetch,
		deadline: u64,
	) -> Result<(Arc<Self>, LogStatus, u64, bool)> {
		let phase = |name| {
			crate::metrics::FS_OPEN_PHASE_DURATION
				.with_label_values(&[name])
				.start_timer()
		};
		let timer = phase("recover");
		let history = rivet_env::timeout_at(clock.as_ref(), deadline, journal.recover())
			.await
			.map_err(|_| Error::DeadlineExceeded)??;
		drop(timer);
		let status = journal.status()?;
		// An empty volume has no cold rows to read and no write to replay. This opening is its
		// sole writer, so the next entry lands right after the claim entry.
		let staged =
			staging && history.empty && status.next_seq == history.committed.saturating_add(1);
		// A volume whose history is all in hand has no cold rows either.
		let (snapshot, prefetched): (Arc<dyn ColdSnapshot>, _) = if staged || history.complete {
			(
				Arc::new(EmptyColdSnapshot {
					position: history.cold.clone(),
				}),
				None,
			)
		} else {
			let timer = phase("cold_open");
			let opened = open_replay_snapshot(
				journal.as_ref(),
				cold_store.as_ref(),
				clock.as_ref(),
				&status.key,
				&history,
				prefetch,
				deadline,
			)
			.await?;
			drop(timer);
			(opened.snapshot, opened.prefetched)
		};
		let start = snapshot.position().clone();
		if start.volume_seq < history.cold.volume_seq
			|| (start.volume_seq == history.cold.volume_seq && start != history.cold)
			|| start.volume_seq > history.committed
			|| history.cold.volume_seq > history.committed
		{
			crate::fences::record(
				crate::fences::FenceReason::OpenColdMismatch,
				Some(&status.key),
				journal.client_epoch(),
				&AtomicBool::new(false),
			);
			journal.fence();
			return Err(corrupt!());
		}
		let budget = Budget::new(config.max_memory_bytes);
		let cache_share = CacheShare::new(process_cache);
		let rows = RowCache::new(
			snapshot.position().clone(),
			budget.clone(),
			config.max_cache_bytes,
			cache_share.clone(),
		);
		let cold = Cold::new(snapshot, rows.clone());
		if let Some(prefetched) = prefetched {
			cold.remember_prefetched(prefetch, prefetched);
		}
		let fs = Arc::new(Self {
			fence_logged: AtomicBool::new(false),
			notes_armed: parking_lot::Mutex::new(None),
			journal,
			cold_store,
			write_gate: Mutex::new(()),
			clock,
			key: status.key.clone(),
			fenced: AtomicBool::new(false),
			undecided_through: AtomicU64::new(0),
			rows,
			cache_share,
			budget,
			state: Mutex::new(State {
				client_epoch: status.client_epoch,
				memory_release_after_boot_ms: 0,
				backlog: Default::default(),
				cold,
				layers: Vec::new(),
				positions: BTreeMap::new(),
				last_seq: start.volume_seq,
				resolver: txn::Resolver::new(start.clone()),
				transactions: BTreeMap::new(),
				next_transaction: 0,
				conflicts: ConflictWindow::default(),
				pins: Vec::new(),
				handles: BTreeMap::new(),
				locks: BTreeMap::new(),
				staged_format: None,
				unobserved_admission: None,
			}),
			queue: parking_lot::Mutex::new(Queue::default()),
			queue_wake: tokio::sync::Notify::new(),
			transaction_wake: tokio::sync::Notify::new(),
			config,
		});
		let committed = history.committed;
		if !staged {
			let mut state = fs.state.lock().await;
			let timer = phase("replay");
			let replayed = fs.replay(&mut state, history, start, deadline).await;
			drop(timer);
			// The log trimmed entries the replay needed; this opening wrote nothing, so it ends
			// without fencing and the caller starts again from a fresh history. A refused replay
			// already recorded why, such as a draining worker revoking dispatch, and this opening
			// is discarded, so fencing it again would only misreport the refusal as a failure.
			if let Err(error) = replayed {
				if error != Error::TooOld && error != Error::Fenced {
					fs.fence_with(crate::fences::FenceReason::ReplayFailed);
				}
				return Err(error);
			}
			// A claim the open settled wrote no entry, so transactions an older writer left
			// open are still open in the view. One abort at this writer's epoch ends them all,
			// the same as the first write of a claim that did not settle.
			let abandoned = state.resolver.open().keys().next().cloned();
			drop(state);
			if let Some(id) = abandoned {
				let timer = phase("abandoned_transactions");
				fs.abort_abandoned(id, deadline).await?;
				drop(timer);
			}
		}
		Ok((fs, status, committed, staged))
	}

	/// Accounts for an empty volume's lone claim entry at sequence 1 without reading it back.
	/// This opening wrote it, so it carries this opening's epoch.
	fn account_claim_entry(&self, state: &mut State, status: &LogStatus) -> Result<()> {
		let claim_entry = rivet_data::generated::depot_log_shard_v1::VolumeLogEntry {
			volume_seq: 1,
			client_epoch: status.client_epoch,
			written_at_unix_ms: 0,
			body: rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::ClaimEntry,
		};
		state.resolver.push(&claim_entry)?;
		state.last_seq = claim_entry.volume_seq;
		state.positions.insert(
			claim_entry.volume_seq,
			(status.client_epoch, self.budget.reserve(256)?),
		);
		Ok(())
	}

	/// Stages the root of an empty volume and the chain of `files` for its first commit,
	/// replacing any earlier staging. The first entry holds the root and the first file; each
	/// further file takes the next entry, because a created inode is numbered by its entry's
	/// sequence. Returns the files' inodes. Their rows sit in sequence 0 layers, so handler
	/// reads see the formatted volume before it is written.
	async fn stage_format(
		&self,
		state: &mut State,
		status: &LogStatus,
		files: &[StagedFile<'_>],
		deadline: u64,
	) -> Result<Vec<u64>> {
		let tree: Vec<_> = files
			.iter()
			.enumerate()
			.map(|(i, file)| (i.checked_sub(1), *file))
			.collect();
		self.stage_format_tree(state, status, &tree, deadline).await
	}

	async fn stage_format_tree(
		&self,
		state: &mut State,
		status: &LogStatus,
		files: &[(Option<usize>, StagedFile<'_>)],
		deadline: u64,
	) -> Result<Vec<u64>> {
		// Validate every parent before replacing the opening's staged view.
		for (index, (parent, _)) in files.iter().enumerate() {
			if parent.is_some_and(|parent| parent >= index || files[parent].1.kind != FsKind::Dir) {
				return Err(depot_fs_core::Error::Invalid.into());
			}
		}
		let now = self.now()?.unix_ms;
		let root = rules::root(0o755, 0, 0, now);
		let first_seq = status.next_seq;
		let mut entries = vec![StagedEntry {
			seq: first_seq,
			ops: vec![root.clone()],
		}];
		state.layers = vec![Note::staged(self.staged_layer(&entries[0])?)];

		let mut inos = Vec::with_capacity(files.len());
		// A created inode is numbered by its entry's sequence, so an entry at the root's own
		// number holds the root alone. That is the first entry of a volume created with its
		// first writes, which has no claim entry before them.
		let first_file_seq = match first_seq == ROOT_INO {
			true => first_seq + 1,
			false => first_seq,
		};
		for (index, (parent_index, file)) in files.iter().enumerate() {
			let parent = parent_index.map_or(ROOT_INO, |index| inos[index]);
			// The first file rides with the root unless the root sits at its own number; every
			// later one opens a new entry.
			let seq = first_file_seq + index as u64;
			let mut view = self.view(state, state.last_seq);
			let create = self
				.evaluate(&mut view, deadline, |v| {
					rules::create(
						v,
						parent,
						file.name.to_vec(),
						rules::new_inode(file.kind, file.mode, 0, 0, 0),
						now,
					)
				})
				.await?;
			drop(view);
			if seq == first_seq {
				entries[0].ops.push(create);
				state.layers = vec![Note::staged(self.staged_layer(&entries[0])?)];
			} else {
				entries.push(StagedEntry {
					seq,
					ops: vec![create],
				});
				let layer = self.staged_layer(entries.last().ok_or_else(|| corrupt!())?)?;
				state.layers.push(Note::staged(layer));
			}
			inos.push(seq);
		}
		state.staged_format = Some(StagedFormat { entries });
		Ok(inos)
	}

	fn staged_layer(&self, entry: &StagedEntry) -> Result<Arc<Layer>> {
		let plan = apply::plan(
			entry.seq,
			&FsEntry {
				txn: None,
				ops: entry.ops.clone(),
			},
		)?;
		super::cache::Layer::staged(entry.seq, plan, &self.budget)
	}

	/// Adds `files` to the staged format of a volume this opening found empty and has not
	/// written yet, and returns their inodes. Returns `None` once the volume holds a commit or
	/// a write was admitted, and the caller looks the files up instead.
	pub async fn stage_files(&self, files: &[StagedFile<'_>]) -> Result<Option<Vec<u64>>> {
		let tree: Vec<_> = files
			.iter()
			.enumerate()
			.map(|(i, file)| (i.checked_sub(1), *file))
			.collect();
		self.stage_tree(&tree).await
	}

	/// Stages a directory tree in a fresh volume without appending. Each parent is the
	/// index of an earlier directory, or `None` for the root; returned inodes match input order.
	/// As with `stage_files`, the next write carries all staged entries in its quorum append.
	pub async fn stage_tree(
		&self,
		files: &[(Option<usize>, StagedFile<'_>)],
	) -> Result<Option<Vec<u64>>> {
		if self.state.lock().await.staged_format.is_none() {
			return Ok(None);
		}
		let deadline = self.deadline()?;
		let _write = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut state = self.state.lock().await;
		let status = self.journal.status()?;
		let unwritten = state.staged_format.as_ref().is_some_and(|staged| {
			staged
				.entries
				.first()
				.is_some_and(|e| e.seq == status.next_seq)
		});
		if !unwritten {
			return Ok(None);
		}
		self.stage_format_tree(&mut state, &status, files, deadline)
			.await
			.map(Some)
	}

	/// Admits the staged format of an empty volume as its own entries, for writers that cannot
	/// carry it in their first entry. It does not wait for them: the writer's own entry follows
	/// them in the log, so its commit makes them durable too, and they ride its append.
	pub(super) async fn write_staged_format(&self) -> Result<()> {
		if self.state.lock().await.staged_format.is_none() {
			return Ok(());
		}
		let deadline = self.deadline()?;
		let _write = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut state = self.state.lock().await;
		if state.staged_format.is_none() {
			return Ok(());
		}
		let status = self.refresh(&mut state, deadline).await?;
		let receipt = self
			.admit(
				&mut state,
				FsEntry {
					txn: None,
					ops: Vec::new(),
				},
				status,
				deadline,
			)
			.await?;
		// A failed append fences this opening through the writer's own entry behind it.
		receipt.attempt.finish();
		Ok(())
	}
	/// Bytes this filesystem holds against its memory budget: write payloads in flight and
	/// overlay layers. Caches are bounded by the process cache budget instead.
	/// Reserves retained derived data against the same process budget as filesystem caches.
	pub fn reserve_cache(&self, bytes: usize) -> Option<super::CacheReservation> {
		self.cache_share.reserve_adapter(bytes)
	}

	/// The injected clock shared by this opening and adapters serving it.
	pub fn clock(&self) -> Arc<dyn Clock> {
		self.clock.clone()
	}

	pub fn memory_bytes(&self) -> usize {
		self.budget.used()
	}

	/// Runs `signal` when a write adds the first pending note, so the owner schedules the
	/// note timer through [`Filesystem::expire_notes`]. A filesystem with no note arms nothing.
	pub fn on_notes_armed(&self, signal: Arc<dyn Fn() + Send + Sync>) {
		*self.notes_armed.lock() = Some(signal);
	}

	/// Adds a written entry's layer to the overlay with its note timer, and signals the owner
	/// when it is the first pending note. Without a clock nothing else serves either, so the
	/// note then waits for a snapshot install.
	pub(super) fn push_layer(&self, state: &mut State, layer: Arc<Layer>) {
		let expires_boot_ms = self.now().map_or(u64::MAX, |now| {
			now.boot_ms.saturating_add(self.config.note_ttl_ms)
		});
		let first = !state.layers.iter().any(Note::timed);
		state.layers.push(Note {
			layer,
			deadline: NoteDeadline::At(expires_boot_ms),
		});
		if first {
			let signal = self.notes_armed.lock().clone();
			if let Some(signal) = signal {
				signal();
			}
		}
	}

	/// The note timer. Drops every committed entry's layer whose notes expired by `now` and
	/// that the maintainer has reported applied, and moves the snapshot's floor to the newest
	/// of them, keeping the rows they held in memory as cache, without a request. A later miss
	/// on one of their rows reads it from the maintainer, which already serves that position.
	///
	/// An expired layer above the newest applied position the maintainer reported is held
	/// instead: it keeps only its index, its page bytes move to the bounded row cache, and a
	/// miss on its rows reads its entry from the log. Moving the floor past it would make that
	/// miss wait for the maintainer's apply, which a slow cold store can hold past any caller's
	/// deadline. The filesystem learns the applied position only from maintainer replies, so a
	/// writer that reads nothing holds its newest layers until a reply, a cold pass or memory
	/// pressure releases them. Past `MAX_HELD_LAYERS` the oldest held layers are dropped as
	/// before, which bounds the overlay every row lookup walks.
	///
	/// Returns when the owner should run it next. Does nothing while a transaction is open,
	/// because transaction reads build their view without a refresh, while a format is staged,
	/// or while an operation holds the filesystem.
	pub fn expire_notes(&self, now: u64) -> NoteExpiry {
		let Ok(mut state) = self.state.try_lock() else {
			return NoteExpiry::Busy;
		};
		let next_retry = now.saturating_add(self.config.note_ttl_ms);
		let Some(first) = state.layers.iter().find(|note| note.timed()) else {
			return NoteExpiry::Idle;
		};
		if let NoteDeadline::At(expires_boot_ms) = first.deadline
			&& expires_boot_ms > now
		{
			return NoteExpiry::Due(expires_boot_ms);
		}
		if !state.transactions.is_empty() || state.staged_format.is_some() {
			return NoteExpiry::Due(next_retry);
		}
		// A journal that is not serving is recovered by the next request, which replays the
		// log anyway; its notes are left alone.
		let Ok(status) = self.status() else {
			return NoteExpiry::Idle;
		};
		let committed = status.committed;
		// Cold never passes the applied position, and the log no longer holds what cold covers.
		let applied = committed
			.min(self.cold_store.observed_applied())
			.max(status.cold.volume_seq.min(committed));
		let mut dropped_through = None;
		let mut to_hold = Vec::new();
		let mut held = 0usize;
		let mut next = None;
		for (index, note) in state.layers.iter().enumerate() {
			let seq = note.layer.seq;
			if seq == 0 {
				continue;
			}
			// Notes are pushed in sequence and time order, so the first one kept bounds the
			// rest. An expired note of an entry still committing waits one more period.
			if let NoteDeadline::At(expires_boot_ms) = note.deadline
				&& next.is_none()
				&& (expires_boot_ms > now || seq > committed)
			{
				next = Some(if expires_boot_ms > now {
					expires_boot_ms
				} else {
					next_retry
				});
			}
			if next.is_some() {
				continue;
			}
			if seq <= applied {
				dropped_through = Some(seq);
				continue;
			}
			held += 1;
			if note.deadline != NoteDeadline::UntilApplied {
				to_hold.push(index);
			}
		}

		// Hold each newly expired unapplied layer as an index. A layer that already holds
		// only an index is held as it is.
		for index in to_hold {
			let note = &state.layers[index];
			match note.layer.index_only(&self.budget, self.journal.clone()) {
				Ok(indexed) => {
					if indexed.is_some() {
						self.rows.remember_entry_rows(&note.layer);
					}
					let note = &mut state.layers[index];
					if let Some(indexed) = indexed {
						note.layer = indexed;
					}
					note.deadline = NoteDeadline::UntilApplied;
				}
				// Without room for the index, the layer is held with its bytes; a memory refresh
				// decides what to release.
				Err(_) => {
					state.layers[index].deadline = NoteDeadline::UntilApplied;
				}
			}
		}

		// Past the cap, the oldest held layers are dropped with the floor move.
		if held > MAX_HELD_LAYERS {
			let excess = held - MAX_HELD_LAYERS;
			if let Some(seq) = state
				.layers
				.iter()
				.filter(|note| note.deadline == NoteDeadline::UntilApplied)
				.nth(excess - 1)
				.map(|note| note.layer.seq)
			{
				dropped_through = dropped_through.max(Some(seq));
			}
		}
		let Some(through) = dropped_through else {
			return next.map_or(NoteExpiry::Idle, NoteExpiry::Due);
		};
		let floor = VolumeLogPosition {
			volume_seq: through,
			client_epoch: entry_epoch(&state, through),
		};
		let trigger = match through <= applied {
			true => "notes",
			false => "notes_unapplied",
		};
		let dropped = self.move_floor(&mut state, floor, trigger);
		crate::metrics::FS_NOTE_EXPIRIES.inc_by(dropped);
		tracing::debug!(
			dropped,
			through,
			committed,
			applied,
			"dropped expired overlay notes; misses on their rows read at or past them"
		);
		next.map_or(NoteExpiry::Idle, NoteExpiry::Due)
	}

	/// Releases the page bytes of every layer of an entry at or below `through`, keeping only
	/// its index, as the note timer does for a held layer. The bytes move to the bounded row
	/// cache.
	fn index_layers_through(&self, state: &mut State, through: u64) {
		for note in state.layers.iter_mut() {
			if note.layer.seq == 0 || note.layer.seq > through {
				continue;
			}
			let Ok(Some(indexed)) = note.layer.index_only(&self.budget, self.journal.clone())
			else {
				continue;
			};
			self.rows.remember_entry_rows(&note.layer);
			note.layer = indexed;
		}
	}

	/// Moves the snapshot to `floor`, a committed position above it, and drops every layer at or
	/// below it with the positions they held. The row cache carries over: the rows the dropped
	/// layers held become rows at the new snapshot. No request is sent; the store's claim
	/// snapshot at the new floor serves the next misses. Returns how many layers it dropped.
	pub(super) fn move_floor(
		&self,
		state: &mut State,
		floor: VolumeLogPosition,
		trigger: &'static str,
	) -> u64 {
		let released: Vec<_> = state
			.layers
			.iter()
			.filter(|note| note.layer.seq <= floor.volume_seq)
			.map(|note| note.layer.clone())
			.collect();
		self.rows.install(&floor, &released);
		let snapshot = self
			.cold_store
			.claim_snapshot(self.key.clone(), floor.clone());
		state.cold = Cold::new(snapshot, self.rows.clone());
		state
			.layers
			.retain(|note| note.layer.seq > floor.volume_seq);
		state.positions.retain(|seq, _| *seq > floor.volume_seq);
		crate::metrics::FS_SNAPSHOT_MOVES
			.with_label_values(&[trigger])
			.inc();
		released.len() as u64
	}

	/// False after [`Filesystem::try_release_idle_caches`] until a read caches a row again.
	pub fn caches_filled(&self) -> bool {
		self.cache_share.filled()
	}

	/// Drops the row cache. It holds only durable copies, so this changes no result. Returns
	/// false without releasing when an operation holds the filesystem or a log read runs; the
	/// caller retries later.
	pub fn try_release_idle_caches(&self) -> bool {
		let Ok(_state) = self.state.try_lock() else {
			return false;
		};
		if !self.rows.try_clear() {
			return false;
		}
		self.cache_share.mark_released();
		crate::metrics::FS_IDLE_CACHE_RELEASES.inc();
		true
	}

	/// Checks this opening and the latest volume-log authority.
	pub fn is_fenced(&self) -> bool {
		self.status().is_err()
	}
	/// Fences this opening for a caller-defined reason; prefer `fence_with` and a named reason.
	pub fn fence(&self) {
		self.fence_with(crate::fences::FenceReason::OwnerReleased);
	}
	/// Fences this opening and its journal, counting `reason` and logging the first cause.
	pub fn fence_with(&self, reason: crate::fences::FenceReason) {
		self.record_fence(reason);
		self.fenced.store(true, Ordering::Release);
		self.journal.fence();
	}
	/// Fences only the journal, so a retryable error can recover through a reopen.
	pub(super) fn fence_journal(&self, reason: crate::fences::FenceReason) {
		self.record_fence(reason);
		self.journal.fence();
	}
	/// Counts `reason` and returns the fenced error, without fencing anything itself.
	pub(super) fn fenced(&self, reason: crate::fences::FenceReason) -> Error {
		self.record_fence(reason);
		Error::Fenced
	}
	fn record_fence(&self, reason: crate::fences::FenceReason) {
		crate::fences::record(
			reason,
			Some(&self.key),
			self.journal.client_epoch(),
			&self.fence_logged,
		);
	}
	pub(super) fn status(&self) -> Result<LogStatus> {
		if self.fenced.load(Ordering::Acquire) {
			return Err(self.fenced(crate::fences::FenceReason::AlreadyFenced));
		}
		let status = self.journal.status()?;
		if status.key != self.key {
			self.fence_with(crate::fences::FenceReason::KeyMismatch);
			return Err(Error::Fenced);
		}
		Ok(status)
	}
	pub(super) fn now(&self) -> Result<rivet_env::Clocks> {
		self.clock.observe().map_err(|_| Error::Unavailable)
	}
	pub(super) fn deadline(&self) -> Result<u64> {
		Ok(self
			.now()?
			.boot_ms
			.saturating_add(self.config.request_timeout_ms))
	}
	pub(super) fn view(&self, state: &State, seq: u64) -> View {
		View::new(
			seq,
			state.cold.clone(),
			state
				.layers
				.iter()
				.filter(|note| note.layer.seq <= seq)
				.map(|note| note.layer.clone())
				.collect(),
			self.budget.clone(),
			self.config.max_operation_bytes,
		)
	}
	/// Rebuilds `view` at `seq` on a newer cold snapshot after one of its reads found the log
	/// already trimmed past an entry the view needed: the entry reached the cold store while
	/// this filesystem was idle, so it never learned the newer cold position. Confirming the
	/// tail learns it, and a view at the same `seq` over the newer snapshot reads the same
	/// state. Returns whether the view was rebuilt; `false` means the cold store has moved
	/// past `seq`, so the view's state no longer exists anywhere.
	pub(super) async fn rebase_view(
		&self,
		view: &mut View,
		seq: u64,
		deadline: u64,
	) -> Result<bool> {
		rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.confirm())
			.await
			.map_err(|_| Error::DeadlineExceeded)??;
		let mut state = self.state.lock().await;
		self.refresh(&mut state, deadline).await?;
		if state.cold.snapshot.position().volume_seq > seq {
			crate::metrics::FS_VIEW_REBASES
				.with_label_values(&["moved_past"])
				.inc();
			return Ok(false);
		}
		*view = self.view(&state, seq);
		crate::metrics::FS_VIEW_REBASES
			.with_label_values(&["rebased"])
			.inc();
		Ok(true)
	}

	/// [`Self::evaluate`] that rebuilds `view` on a newer cold snapshot when the log trimmed
	/// an entry the view needed; see [`Self::rebase_trimmed`].
	pub(super) async fn evaluate_rebasing<T>(
		&self,
		view: &mut View,
		deadline: u64,
		f: impl Fn(&View) -> depot_fs_core::Result<T>,
	) -> Result<T> {
		let result = self.evaluate(view, deadline, &f).await;
		self.rebase_trimmed(view, deadline, &f, result).await
	}

	/// Turns a `TooOld` evaluation of `view` into a rebuild over the newer cold snapshot and
	/// a new evaluation, as often as cold keeps moving under it, until the deadline. Tiering
	/// can offload and trim again between one rebuild's confirm and its evaluation, so a
	/// single rebuild is not enough under frequent offloads. Each further rebuild needs
	/// evidence: the rebuilt view's cold position must be newer than before. Cold never passes
	/// `view.seq` while the view's state still exists, so the rebuilds are finite. One rebuild
	/// without evidence is allowed, because a moved maintainer can refuse a position cold has
	/// not passed. Any other result is returned as it is.
	pub(super) async fn rebase_trimmed<T>(
		&self,
		view: &mut View,
		deadline: u64,
		f: impl Fn(&View) -> depot_fs_core::Result<T>,
		mut result: Result<T>,
	) -> Result<T> {
		let mut rebased_without_evidence = false;
		loop {
			if !matches!(result, Err(Error::TooOld)) {
				return result;
			}
			let cold_before = view.cold.snapshot.position().volume_seq;
			let seq = view.seq;
			if !self.rebase_view(view, seq, deadline).await? {
				return Err(Error::TooOld);
			}
			if view.cold.snapshot.position().volume_seq <= cold_before {
				if rebased_without_evidence {
					return Err(Error::TooOld);
				}
				rebased_without_evidence = true;
			}
			result = self.evaluate(view, deadline, &f).await;
		}
	}

	pub(super) async fn evaluate<T>(
		&self,
		view: &mut View,
		deadline: u64,
		f: impl Fn(&View) -> depot_fs_core::Result<T>,
	) -> Result<T> {
		loop {
			self.status()?;
			if self.now()?.boot_ms >= deadline {
				return Err(Error::DeadlineExceeded);
			}
			view.start_evaluation();
			match f(view) {
				Ok(value) => {
					self.status()?;
					if self.now()?.boot_ms >= deadline {
						return Err(Error::DeadlineExceeded);
					}
					view.record_served();
					return Ok(value);
				}
				Err(depot_fs_core::Error::ReadRequired(request)) => {
					let result = rivet_env::timeout_at(
						self.clock.as_ref(),
						deadline,
						view.load(request, deadline),
					)
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
					if matches!(
						result,
						Err(Error::Corrupt | Error::Filesystem(depot_fs_core::Error::Corrupt))
					) {
						self.fence_with(crate::fences::FenceReason::Corrupt);
					}
					result?;
				}
				Err(depot_fs_core::Error::Corrupt) => {
					self.fence_with(crate::fences::FenceReason::Corrupt);
					return Err(corrupt!());
				}
				Err(error) => return Err(error.into()),
			}
		}
	}
	/// Moves the snapshot forward when that releases memory or follows cold, and returns the
	/// log status it judged against.
	///
	/// The snapshot can sit above cold, at a committed position of this filesystem's own
	/// history, so cold below the snapshot is normal. Moving it is local: it drops every layer
	/// at or below the new floor and sends nothing, and the misses that follow wait for the
	/// maintainer to apply that floor. An idle volume sends nothing.
	pub(super) async fn refresh(&self, state: &mut State, deadline: u64) -> Result<LogStatus> {
		self.refresh_inner(state, deadline, true).await
	}

	/// [`Self::refresh`] for a request whose view stops at the committed position, which may
	/// therefore serve while an undecided commit is still settling instead of waiting for it.
	pub(super) async fn refresh_committed(
		&self,
		state: &mut State,
		deadline: u64,
	) -> Result<LogStatus> {
		self.refresh_inner(state, deadline, false).await
	}

	/// The refresh; `settle` makes it wait for undecided commits first.
	async fn refresh_inner(
		&self,
		state: &mut State,
		deadline: u64,
		settle: bool,
	) -> Result<LogStatus> {
		let status = self.current(state, deadline).await?;
		let status = self
			.reconcile_unapplied_with(state, status, deadline, settle)
			.await?;
		let current = state.cold.snapshot.position().clone();
		if status.cold.volume_seq == current.volume_seq && status.cold != current {
			self.fence_journal(crate::fences::FenceReason::ColdEpochMismatch);
			return Err(Error::Fenced);
		}
		let cold_passed = status.cold.volume_seq > current.volume_seq;
		if cold_passed && entry_epoch(state, status.cold.volume_seq) != status.cold.client_epoch {
			self.fence_journal(crate::fences::FenceReason::ColdEpochMismatch);
			return Err(Error::Fenced);
		}
		let now = self.now()?.boot_ms;
		// Layers above the snapshot hold an index of written keys in this filesystem's memory
		// budget. Once an eighth of it is used, moving the floor releases them. The floor goes
		// to the newest layer the maintainer has reported applied, so the misses after the
		// move do not wait for an apply. Only from half the budget, and only when no applied
		// layer would be released, does it go as far as the committed position. Below half
		// the budget this runs at most once a second, and from half every 100 ms. Nothing
		// here asks for a write to cold.
		let committed = status.committed.min(state.last_seq);
		let pressed = self.budget.half_used();
		let newest_layer_through = |limit: u64| {
			state
				.layers
				.iter()
				.map(|note| note.layer.seq)
				.filter(|seq| *seq > current.volume_seq && *seq <= limit)
				.max()
		};
		let mut memory_floor =
			match self.budget.eighth_used() && now >= state.memory_release_after_boot_ms {
				true => {
					let applied =
						newest_layer_through(committed.min(self.cold_store.observed_applied()));
					match (applied, pressed) {
						(Some(seq), _) => Some(seq),
						(None, true) => newest_layer_through(committed),
						(None, false) => None,
					}
				}
				false => None,
			};
		// Under pressure with nothing applied to move to, the committed layers give up their
		// page bytes and keep only their index, so their rows are read from the log instead of
		// from a maintainer whose apply lags. Only a budget about to run out moves the floor past
		// them as before, since the next misses then wait for the apply; commit admission's
		// backlog cap normally refuses the writer long before.
		if let Some(seq) = memory_floor
			&& seq > committed.min(self.cold_store.observed_applied())
		{
			if state.staged_format.is_none() {
				self.index_layers_through(state, seq);
			}
			if !self.budget.nearly_full() {
				memory_floor = None;
				state.memory_release_after_boot_ms = now.saturating_add(PRESSED_RELEASE_MS);
			}
		}
		// Held layers whose entries the maintainer has since reported applied are released
		// with a floor move to the newest of them, as the note timer would have.
		let applied = committed.min(self.cold_store.observed_applied());
		let held_floor = state
			.layers
			.iter()
			.filter(|note| note.layer.seq > current.volume_seq)
			.take_while(|note| {
				note.deadline == NoteDeadline::UntilApplied && note.layer.seq <= applied
			})
			.map(|note| note.layer.seq)
			.last();
		// While a format is staged, its rows exist only in the sequence 0 layer, which a move
		// would drop.
		if state.staged_format.is_none() {
			if let Some(seq) = held_floor
				&& memory_floor.is_none_or(|memory| memory <= seq)
				&& seq >= status.cold.volume_seq
			{
				let floor = match seq == status.cold.volume_seq {
					true => status.cold.clone(),
					false => VolumeLogPosition {
						volume_seq: seq,
						client_epoch: entry_epoch(state, seq),
					},
				};
				self.move_floor(state, floor, "applied");
			} else if let Some(seq) = memory_floor
				&& seq >= status.cold.volume_seq
			{
				let floor = match seq == status.cold.volume_seq {
					true => status.cold.clone(),
					false => VolumeLogPosition {
						volume_seq: seq,
						client_epoch: entry_epoch(state, seq),
					},
				};
				// A move past the applied position is counted apart: misses after it wait for
				// the maintainer's apply.
				let trigger = match seq <= applied {
					true => "memory",
					false => "memory_unapplied",
				};
				self.move_floor(state, floor, trigger);
				let gap = match pressed {
					true => PRESSED_RELEASE_MS,
					false => MEMORY_RELEASE_MS,
				};
				state.memory_release_after_boot_ms = now.saturating_add(gap);
			} else if cold_passed {
				// Cold passed the snapshot, so the log no longer holds the entries between them;
				// the layers for them are released with the move.
				self.move_floor(state, status.cold.clone(), "cold");
			}
		}
		state.pins.retain(|weak| {
			weak.upgrade().is_some_and(|pin| {
				if now >= pin.expires_boot_ms {
					if let Ok(mut view) = pin.view.try_lock() {
						*view = None;
					}
					false
				} else {
					true
				}
			})
		});
		Ok(status)
	}

	pub(super) async fn read_with<T>(
		&self,
		f: impl Fn(&View) -> depot_fs_core::Result<T>,
	) -> Result<T> {
		self.read_with_chunks(None, f).await
	}

	/// Like [`Self::read_with`], first loading the chunks of `ino` in `spans`, each
	/// `(first, count)`, that the read will need, in one round of cold requests.
	async fn read_with_chunks<T>(
		&self,
		chunks: Option<(u64, Vec<(u64, u64)>)>,
		f: impl Fn(&View) -> depot_fs_core::Result<T>,
	) -> Result<T> {
		let deadline = self.deadline()?;
		let mut refresh_cold = false;
		loop {
			let result = async {
				if self.now()?.boot_ms >= deadline {
					return Err(Error::DeadlineExceeded);
				}
				if refresh_cold {
					rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.confirm())
						.await
						.map_err(|_| Error::DeadlineExceeded)??;
				}
				let mut state = self.state.lock().await;
				let status = self.refresh_committed(&mut state, deadline).await?;
				let mut view = self.view(&state, state.last_seq.min(status.committed));
				drop(state);
				if let Some((ino, spans)) = &chunks {
					let prefetched = rivet_env::timeout_at(
						self.clock.as_ref(),
						deadline,
						view.prefetch_chunks(*ino, spans, deadline),
					)
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
					if matches!(
						prefetched,
						Err(Error::Corrupt | Error::Filesystem(depot_fs_core::Error::Corrupt))
					) {
						self.fence_with(crate::fences::FenceReason::Corrupt);
					}
					prefetched?;
				}
				self.evaluate(&mut view, deadline, &f).await
			}
			.await;
			if matches!(result, Err(Error::TooOld)) {
				// Latest reads may advance to cold after trim. Never rebuild the
				// owner or invalidate unrelated transactions for an expired view.
				if refresh_cold {
					let wake = self.now()?.boot_ms.saturating_add(20).min(deadline);
					self.clock
						.sleep_until(wake)
						.await
						.map_err(|_| Error::Unavailable)?;
				}
				refresh_cold = true;
				continue;
			}
			return result;
		}
	}

	/// The closure resolves postimages using the current admitted overlay. It may
	/// be evaluated again after cold misses, so it must have no side effects.
	pub(super) async fn mutate(
		&self,
		build: impl Fn(&View, u64, i64) -> depot_fs_core::Result<Vec<FsOp>>,
	) -> Result<u64> {
		match self.mutate_entry(build).await? {
			MutationOutcome::Committed(committed) => Ok(committed.volume_seq),
			MutationOutcome::Unchanged(seq) => Ok(seq),
		}
	}
	async fn mutate_entry(
		&self,
		build: impl Fn(&View, u64, i64) -> depot_fs_core::Result<Vec<FsOp>>,
	) -> Result<MutationOutcome> {
		let deadline = self.deadline()?;
		let mut confirm_cold = false;
		// The cold position the last write attempt that hit the log's trim was built on.
		let mut trimmed_at_cold: Option<u64> = None;
		// The cold position the current attempt's view was built on.
		let mut built_on_cold = 0;
		loop {
			let wake = self.transaction_wake.notified();
			tokio::pin!(wake);
			wake.as_mut().enable();
			let write =
				rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
			let confirm_now = std::mem::take(&mut confirm_cold);
			let prepared = async {
				if confirm_now {
					rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.confirm())
						.await
						.map_err(|_| Error::DeadlineExceeded)??;
				}
				let mut state = self.state.lock().await;
				let status = self.refresh(&mut state, deadline).await?;
				let expected_next = state.last_seq.checked_add(1).ok_or_else(|| corrupt!())?;
				if status.next_seq < expected_next {
					return Err(corrupt!());
				}
				// The refresh installed every entry this opening admitted without observing
				// its outcome, so the log can hold no entry above the view here.
				if status.next_seq > expected_next {
					self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
					return Err(Error::Fenced);
				}
				self.shed_unapplied_backlog(&mut state, deadline).await?;
				let mut view = self.view(&state, state.last_seq);
				built_on_cold = view.cold.snapshot.position().volume_seq;
				let ops = self
					.evaluate(&mut view, deadline, |v| {
						build(
							v,
							status.next_seq,
							self.now()
								.map_err(|_| depot_fs_core::Error::Corrupt)?
								.unix_ms,
						)
					})
					.await?;
				// An empty entry still writes a staged format.
				if ops.is_empty() && state.staged_format.is_none() {
					return Ok(None);
				}
				self.admit(&mut state, FsEntry { txn: None, ops }, status, deadline)
					.await
					.map(Some)
			}
			.await;
			// A trimmed read is worth building again while each confirm teaches the filesystem
			// a newer cold position: the first trim always confirms, and each later one only
			// when its attempt was built on a newer cold position than the previous trimmed
			// attempt. The deadline bounds how often tiering can move it.
			let mut cold_moved = false;
			if matches!(prepared, Err(Error::TooOld)) {
				cold_moved = trimmed_at_cold.is_none_or(|previous| built_on_cold > previous);
				trimmed_at_cold = Some(built_on_cold);
			}
			match prepared {
				Ok(Some(receipt)) => {
					return self
						.finish_write(receipt, deadline)
						.await
						.map(MutationOutcome::Committed);
				}
				Ok(None) => {
					return Ok(MutationOutcome::Unchanged(self.state.lock().await.last_seq));
				}
				Err(Error::Busy) => {
					drop(write);
					rivet_env::timeout_at(self.clock.as_ref(), deadline, wake)
						.await
						.map_err(|_| Error::DeadlineExceeded)?;
				}
				// The log offloaded and trimmed an entry the view reads while this filesystem
				// had not learned the newer cold position. Nothing was admitted yet, so
				// confirming the tail and building the view again over the newer cold
				// snapshot evaluates the same state. Cold can move again before the rebuilt
				// view reads, so this repeats while each attempt was built on a newer cold
				// position. A `TooOld` on an unchanged cold position is not explained by
				// tiering and is returned.
				Err(Error::TooOld) if cold_moved => {
					confirm_cold = true;
				}
				Err(error) => return Err(error),
			}
		}
	}

	/// Stops admission, drains the captured commit barrier, then sends VolumeClose. Nothing is
	/// held on the maintainer to release. The separate volume owner remains responsible for
	/// joining its runner tasks.
	pub async fn close(&self) -> Result<()> {
		let deadline = self.deadline()?;
		let mut state = self.state.lock().await;
		self.current(&mut state, deadline).await?;
		let ticket = self.journal.barrier()?;
		self.fenced.store(true, Ordering::Release);
		let attempt = Attempt::new(self);
		rivet_env::timeout_at(self.clock.as_ref(), deadline, ticket)
			.await
			.map_err(|_| Error::DeadlineExceeded)?
			.map_err(Error::Log)?;
		// The barrier made every admitted write durable on a quorum, so nothing below can
		// leave an outcome unknown.
		attempt.finish();
		rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.close())
			.await
			.map_err(|_| Error::DeadlineExceeded)??;
		self.rows
			.reset(state.cold.snapshot.position().clone())
			.await;
		state.layers.clear();
		state.transactions.clear();
		self.transaction_wake.notify_waiters();
		state.positions.clear();
		state.handles.clear();
		state.locks.clear();
		for pin in state.pins.drain(..).filter_map(|pin| pin.upgrade()) {
			if let Ok(mut view) = pin.view.try_lock() {
				*view = None;
			}
		}
		Ok(())
	}

	pub async fn fsync(&self) -> Result<()> {
		let deadline = self.deadline()?;
		let _write = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		self.recover_before(deadline).await?;
		let ticket = self.journal.barrier()?;
		rivet_env::timeout_at(self.clock.as_ref(), deadline, ticket)
			.await
			.map_err(|_| Error::DeadlineExceeded)?
			.map_err(Error::Log)?;
		Ok(())
	}

	pub async fn lookup(&self, parent: u64, name: &[u8]) -> Result<FsDentry> {
		self.read_with(|v| rules::lookup(v, parent, name, None))
			.await
	}
	pub async fn getattr(&self, ino: u64) -> Result<super::Inode> {
		self.read_with(|v| v.inode(ino)).await
	}
	pub async fn resolve(&self, start: u64, path: &[u8], follow_final: bool) -> Result<u64> {
		self.read_with(|v| rules::resolve(v, start, path, follow_final, None))
			.await
	}
	pub async fn readdir(
		&self,
		ino: u64,
		after: Option<&[u8]>,
		limit: usize,
	) -> Result<Vec<(Vec<u8>, FsDentry)>> {
		self.read_with(|v| rules::readdir(v, ino, after, limit, None))
			.await
	}
	pub async fn read(&self, ino: u64, offset: u64, count: usize) -> Result<Vec<u8>> {
		let mut ranges = self.read_ranges(ino, &[(offset, count)]).await?;
		ranges.pop().ok_or_else(|| corrupt!())
	}

	/// Reads several ranges of `ino`, each `(offset, count)`, together: their chunks come
	/// from the cold store in one round of requests instead of one per range. Returns the
	/// ranges' bytes in order. A range past the end of the file reads short, as [`Self::read`]
	/// does. Ranges are read in batches whose chunks fill at most half of
	/// `max_operation_bytes`, each a read of its own, so a batch's view also has room for the
	/// keys and the inode rows a read charges beside its chunks. Up to `READ_BATCHES_IN_FLIGHT`
	/// batches run at once, which bounds what one read holds of the memory budget.
	pub async fn read_ranges(&self, ino: u64, ranges: &[(u64, usize)]) -> Result<Vec<Vec<u8>>> {
		let chunk = depot_fs_core::CHUNK_BYTES as u64;
		let batch_limit = self.config.max_operation_bytes / 2;
		let mut batches: Vec<(Vec<usize>, Vec<(u64, u64)>, usize)> = Vec::new();
		for (index, &(offset, count)) in ranges.iter().enumerate() {
			if count > self.config.max_operation_bytes {
				return Err(Error::TooLarge);
			}
			let first = offset / chunk;
			let end = offset.saturating_add(count as u64).div_ceil(chunk);
			let span = (first, end.saturating_sub(first));
			let bytes = (span.1 as usize).saturating_mul(depot_fs_core::CHUNK_BYTES);
			match batches.last_mut() {
				Some((indices, spans, batch_bytes))
					if batch_bytes.saturating_add(bytes) <= batch_limit =>
				{
					indices.push(index);
					spans.push(span);
					*batch_bytes += bytes;
				}
				Some(_) | None => batches.push((vec![index], vec![span], bytes)),
			}
		}
		// Each batch's read owns what it needs, so the futures carry no borrows of this call.
		let mut reads = Vec::with_capacity(batches.len());
		for (indices, spans, _) in &batches {
			let batch_ranges: Vec<(u64, usize)> =
				indices.iter().map(|&index| ranges[index]).collect();
			reads.push(self.read_with_chunks(Some((ino, spans.clone())), move |v| {
				batch_ranges
					.iter()
					.map(|&(offset, count)| rules::read(v, ino, offset, count, None))
					.collect::<depot_fs_core::Result<Vec<Vec<u8>>>>()
			}));
		}
		let replies: Vec<Vec<Vec<u8>>> = futures_util::stream::iter(reads)
			.buffered(READ_BATCHES_IN_FLIGHT)
			.try_collect()
			.await?;
		let mut results: Vec<Option<Vec<u8>>> = vec![None; ranges.len()];
		for ((indices, _, _), bytes) in batches.iter().zip(replies) {
			for (&index, bytes) in indices.iter().zip(bytes) {
				results[index] = Some(bytes);
			}
		}
		results
			.into_iter()
			.map(|bytes| bytes.ok_or_else(|| corrupt!()))
			.collect()
	}
	pub async fn write(&self, ino: u64, offset: u64, bytes: &[u8]) -> Result<u64> {
		if bytes.len() > self.config.max_operation_bytes {
			return Err(Error::TooLarge);
		}
		self.mutate(|v, _, now| {
			Ok(rules::write(v, ino, offset, bytes, now)?
				.into_iter()
				.collect())
		})
		.await
	}
	pub async fn create(&self, parent: u64, name: &[u8], inode: FsInode) -> Result<u64> {
		self.mutate(|v, _, now| {
			Ok(vec![rules::create(
				v,
				parent,
				name.to_vec(),
				inode.clone(),
				now,
			)?])
		})
		.await
	}
	pub async fn mkdir(
		&self,
		parent: u64,
		name: &[u8],
		mode: u32,
		uid: u32,
		gid: u32,
	) -> Result<u64> {
		self.create(
			parent,
			name,
			rules::new_inode(FsKind::Dir, mode, uid, gid, self.now()?.unix_ms),
		)
		.await
	}
	pub async fn symlink(
		&self,
		parent: u64,
		name: &[u8],
		target: &[u8],
		uid: u32,
		gid: u32,
	) -> Result<u64> {
		let mut inode = rules::new_inode(FsKind::Symlink, 0o777, uid, gid, self.now()?.unix_ms);
		inode.target = target.to_vec().into();
		self.create(parent, name, inode).await
	}
	pub async fn mknod(
		&self,
		parent: u64,
		name: &[u8],
		kind: FsKind,
		mode: u32,
		rdev: u64,
	) -> Result<u64> {
		if matches!(
			kind,
			FsKind::File | FsKind::Dir | FsKind::Symlink | FsKind::Kv
		) {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		let mut inode = rules::new_inode(kind, mode, 0, 0, self.now()?.unix_ms);
		inode.rdev = rdev;
		self.create(parent, name, inode).await
	}
	pub async fn readlink(&self, ino: u64) -> Result<Vec<u8>> {
		let inode = self.getattr(ino).await?;
		if inode.kind != FsKind::Symlink {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		Ok(inode.inode.target.to_vec())
	}
	pub async fn link(&self, ino: u64, parent: u64, name: &[u8]) -> Result<u64> {
		self.mutate(|v, _, now| Ok(vec![rules::link(v, ino, parent, name.to_vec(), now)?]))
			.await
	}
	pub async fn unlink(&self, parent: u64, name: &[u8]) -> Result<u64> {
		self.remove(parent, name, false).await
	}
	pub async fn rmdir(&self, parent: u64, name: &[u8]) -> Result<u64> {
		self.remove(parent, name, true).await
	}
	async fn remove(&self, parent: u64, name: &[u8], directory: bool) -> Result<u64> {
		let seq = self
			.mutate(|v, _, now| {
				Ok(vec![rules::unlink(
					v,
					parent,
					name.to_vec(),
					directory,
					now,
				)?])
			})
			.await?;
		self.reclaim_closed().await?;
		Ok(seq)
	}
	pub async fn rename(&self, from: u64, name: &[u8], to: u64, new_name: &[u8]) -> Result<u64> {
		let seq = self
			.mutate(|v, _, now| {
				Ok(
					rules::rename(v, from, name.to_vec(), to, new_name.to_vec(), now)?
						.into_iter()
						.collect(),
				)
			})
			.await?;
		self.reclaim_closed().await?;
		Ok(seq)
	}
	pub async fn setattr(&self, ino: u64, attributes: rules::Attributes) -> Result<u64> {
		self.mutate(|v, _, now| Ok(vec![rules::setattr(v, ino, attributes.clone(), now)?]))
			.await
	}
	pub async fn getxattr(&self, ino: u64, name: &[u8]) -> Result<Vec<u8>> {
		self.read_with(|v| {
			store::inode(v, ino, None)?;
			match v.get(&RowKey::Xattr(ino, name.to_vec()), None)? {
				Some(Row::Bytes(x)) => Ok(Vec::from(x)),
				None => Err(depot_fs_core::Error::NoAttribute),
				_ => Err(depot_fs_core::Error::Corrupt),
			}
		})
		.await
	}
	pub async fn listxattr(&self, ino: u64) -> Result<Vec<Vec<u8>>> {
		self.read_with(|v| {
			store::inode(v, ino, None)?;
			let mut names = Vec::new();
			let mut cursor = None;
			loop {
				let page = v.scan(&RowRange::Xattrs(ino), cursor.as_ref(), 256, None)?;
				if page.is_empty() {
					break;
				}
				cursor = page.last().map(|(key, _)| key.clone());
				for (key, _) in page {
					if let RowKey::Xattr(_, name) = key {
						names.push(name);
					}
				}
			}
			Ok(names)
		})
		.await
	}
	pub async fn setxattr(
		&self,
		ino: u64,
		name: &[u8],
		value: &[u8],
		create_only: bool,
		replace_only: bool,
	) -> Result<u64> {
		if value.len() > self.config.max_value_bytes {
			return Err(Error::TooLarge);
		}
		self.mutate(|v, _, now| {
			Ok(vec![rules::setxattr(
				v,
				ino,
				name.to_vec(),
				value.to_vec(),
				create_only,
				replace_only,
				now,
			)?])
		})
		.await
	}
	pub async fn removexattr(&self, ino: u64, name: &[u8]) -> Result<u64> {
		self.mutate(|v, _, now| Ok(vec![rules::removexattr(v, ino, name.to_vec(), now)?]))
			.await
	}
	pub async fn open_handle(&self, ino: u64) -> Result<()> {
		let deadline = self.deadline()?;
		let mut state = self.state.lock().await;
		self.refresh(&mut state, deadline).await?;
		let mut view = self.view(&state, state.last_seq);
		self.evaluate(&mut view, deadline, |v| store::inode(v, ino, None))
			.await?;
		*state.handles.entry(ino).or_default() += 1;
		Ok(())
	}
	pub async fn close_handle(&self, ino: u64, owner: u64) -> Result<()> {
		let mut state = self.state.lock().await;
		self.current(&mut state, self.deadline()?).await?;
		let count = state
			.handles
			.get_mut(&ino)
			.ok_or(depot_fs_core::Error::Invalid)?;
		*count -= 1;
		if *count == 0 {
			state.handles.remove(&ino);
		}
		if let Some(locks) = state.locks.get_mut(&ino) {
			locks.retain(|l| l.owner != owner);
		}
		drop(state);
		self.reclaim_closed().await
	}
	pub async fn lock(&self, ino: u64, lock: FileLock) -> Result<()> {
		let mut state = self.state.lock().await;
		self.current(&mut state, self.deadline()?).await?;
		if !state.handles.contains_key(&ino) || lock.end.is_some_and(|end| end <= lock.start) {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		let locks = state.locks.entry(ino).or_default();
		if locks.iter().any(|old| old.conflicts(&lock)) {
			return Err(Error::Conflict);
		}

		locks.push(lock);
		Ok(())
	}
	/// Atomically upgrades, downgrades or releases one open handle's lock set.
	/// On conflict the previous locks remain held.
	pub async fn replace_locks(
		&self,
		ino: u64,
		owner: u64,
		replacement: &[FileLock],
	) -> Result<()> {
		let mut state = self.state.lock().await;
		self.current(&mut state, self.deadline()?).await?;
		if !state.handles.contains_key(&ino)
			|| replacement
				.iter()
				.any(|lock| lock.owner != owner || lock.end.is_some_and(|end| end <= lock.start))
		{
			return Err(depot_fs_core::Error::Invalid.into());
		}
		let locks = state.locks.entry(ino).or_default();
		if replacement
			.iter()
			.any(|lock| locks.iter().any(|old| old.conflicts(lock)))
		{
			return Err(Error::Conflict);
		}
		locks.retain(|lock| lock.owner != owner);
		locks.extend_from_slice(replacement);
		Ok(())
	}
	pub async fn lock_conflicts(&self, ino: u64, lock: &FileLock) -> Result<bool> {
		let mut state = self.state.lock().await;
		self.current(&mut state, self.deadline()?).await?;
		Ok(state
			.locks
			.get(&ino)
			.is_some_and(|locks| locks.iter().any(|old| old.conflicts(lock))))
	}
	pub async fn unlock(&self, ino: u64, owner: u64) -> Result<()> {
		let mut state = self.state.lock().await;
		self.current(&mut state, self.deadline()?).await?;
		if let Some(locks) = state.locks.get_mut(&ino) {
			locks.retain(|l| l.owner != owner);
		}
		Ok(())
	}
	async fn reclaim_closed(&self) -> Result<()> {
		let _write = self.write_gate.lock().await;
		// Each candidate is rechecked while holding the admission lock. An open
		// racing unlink cannot be reclaimed underneath its newly acquired handle.
		let deadline = self.deadline()?;
		loop {
			let mut state = self.state.lock().await;
			let status = self.refresh(&mut state, deadline).await?;
			let mut view = self.view(&state, state.last_seq);
			let candidate = self
				.evaluate(&mut view, deadline, |v| {
					let mut cursor = None;
					loop {
						let page = v.scan(&RowRange::Orphans, cursor.as_ref(), 128, None)?;
						if page.is_empty() {
							return Ok(None);
						}
						cursor = page.last().map(|(k, _)| k.clone());
						for (key, _) in page {
							if let RowKey::Orphan(ino) = key
								&& !state.handles.contains_key(&ino)
							{
								return Ok(Some(ino));
							}
						}
					}
				})
				.await?;
			let Some(ino) = candidate else { return Ok(()) };
			let entry = FsEntry {
				txn: None,
				ops: vec![FsOp::FsReclaim(FsReclaim { ino })],
			};
			let receipt = self.admit(&mut state, entry, status, deadline).await?;
			drop(state);
			self.finish_write(receipt, deadline).await?;
		}
	}
}

/// The client epoch of the entry at `seq` above the snapshot. Entries another writer wrote
/// keep their epoch in `positions`; every other entry there is this filesystem's own, because
/// only its journal admits entries above the ones it replayed, and the note timer drops only
/// its own positions.
pub(super) fn entry_epoch(state: &State, seq: u64) -> u64 {
	state
		.positions
		.get(&seq)
		.map_or(state.client_epoch, |(epoch, _)| *epoch)
}

/// The shortest time between two floor moves the memory budget triggers.
const MEMORY_RELEASE_MS: u64 = 1_000;

/// Most expired layers a filesystem holds as an index above its snapshot while the maintainer
/// has not reported them applied. A row lookup that no layer answers checks each of them, so
/// this bounds that cost. Commit admission refuses writes before a writer gets this far ahead.
pub(super) const MAX_HELD_LAYERS: usize = 1024;

/// The shortest time between two floor moves the memory budget triggers once it is half used.
const PRESSED_RELEASE_MS: u64 = 100;
