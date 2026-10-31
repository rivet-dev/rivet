//! Injected log and cold-row read boundaries. Construction starts no tasks.

use super::{Error, Result};
use depot_fs_core::store::{Row, RowKey, RowRange};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::{
	runner::{CommitTicket, LogHistory, VolumeHandle},
	state::VolumeClientPhase,
	types::{WriteOutcome, WriteToken},
};
use futures_util::future::BoxFuture;
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use rivet_topology::versions::WriteVersions;
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use vbare::Bytes;

#[derive(Clone, Debug)]
pub struct LogStatus {
	pub key: VolumeLogKey,
	pub cold: VolumeLogPosition,
	pub committed: u64,
	pub next_seq: u64,
	pub client_epoch: u64,
	pub max_payload_bytes: usize,
	pub retained_bytes: usize,
}

/// One owner must serialize filesystem admission on this journal. Uncertain outcomes
/// fence that owner; retries are handled by volume recovery, not by re-applying an op.
pub trait Journal: Send + Sync {
	fn spawn_cleanup(&self, _cleanup: BoxFuture<'static, ()>) -> Result<()> {
		Err(Error::Unavailable)
	}
	fn status(&self) -> Result<LogStatus>;
	/// The `depot.fs_entry` version the next admitted entry is encoded at. It comes from the
	/// worker's topology, so a rolling deploy never writes an entry a live log shard or
	/// maintainer cannot read.
	fn entry_version(&self) -> Result<u16>;
	/// Whether `read` serves this journal's own admitted entries before they commit, so a
	/// reader of one needs no commit barrier first.
	fn reads_admitted(&self) -> bool {
		false
	}
	/// Reconcile the original mutation and await durability. Success retains its
	/// committed position (or a cold floor covering it), independently of cache reads.
	fn retry(&self, _payload: Bytes, _token: WriteToken) -> BoxFuture<'_, Result<WriteOutcome>> {
		Box::pin(async { Err(Error::Fenced) })
	}
	fn recover(&self) -> BoxFuture<'_, Result<LogHistory>>;
	/// Bounded replay and transaction-view misses. An empty page below committed
	/// must carry an advanced cold position or is corrupt.
	fn read(&self, first: u64, max_bytes: usize) -> BoxFuture<'_, Result<LogHistory>> {
		Box::pin(async move {
			let mut history = self.recover().await?;
			let mut bytes = 0usize;
			history.entries.retain(|e| {
				if e.volume_seq < first {
					return false;
				}
				bytes = bytes.saturating_add(depot_types::log_shard::entry_bytes(e) as usize);
				bytes <= max_bytes
			});
			Ok(history)
		})
	}

	fn admit(&self, payload: Bytes) -> BoxFuture<'_, Result<WriteOutcome>>;
	/// A ticket for the commit of every entry admitted before this call. The journal never
	/// changes volume client under its filesystem, so the ticket needs no binding to the
	/// admitting history: a journal that stopped serving fails it.
	fn barrier(&self) -> Result<CommitTicket>;
	fn confirm(&self) -> BoxFuture<'_, Result<u64>> {
		Box::pin(async move { self.barrier()?.await.map_err(Error::Log) })
	}
	fn close(&self) -> BoxFuture<'_, Result<()>>;
	fn fence(&self);
	/// The client epoch of the volume client behind this journal, for fence diagnostics.
	fn client_epoch(&self) -> Option<u64> {
		None
	}
	/// Waits until the volume client's published state changes after this call, such as its
	/// committed position or phase. The wait starts when it is called, not when it is first
	/// polled, so callers create it before the request whose refusal it waits out. Callers
	/// bound it with their request deadline.
	fn wait_progress(&self) -> BoxFuture<'_, Result<()>> {
		Box::pin(async { Err(Error::Unavailable) })
	}
}

pub struct VolumeJournal {
	handle: parking_lot::RwLock<VolumeHandle>,
	may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
	versions: Arc<dyn WriteVersions>,
	fenced: AtomicBool,
	/// Whether this journal logged its first fence cause.
	fence_logged: AtomicBool,
}

impl VolumeJournal {
	pub fn new(
		handle: VolumeHandle,
		may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
		versions: Arc<dyn WriteVersions>,
	) -> Self {
		Self {
			handle: parking_lot::RwLock::new(handle),
			may_serve,
			versions,
			fenced: AtomicBool::new(false),
			fence_logged: AtomicBool::new(false),
		}
	}
}

impl VolumeJournal {
	fn handle(&self) -> VolumeHandle {
		self.handle.read().clone()
	}
	/// Counts `reason` and returns the fenced error, logging the first cause with `phase`.
	fn fenced(
		&self,
		reason: crate::fences::FenceReason,
		phase: Option<&dyn std::fmt::Debug>,
	) -> Error {
		let handle = self.handle();
		let view = handle.snapshot.borrow();
		crate::fences::record_with(
			reason,
			view.volume_log_key.as_deref(),
			Some(view.client.client_epoch),
			phase,
			&self.fence_logged,
		);
		Error::Fenced
	}
}

/// The version an encoded payload carries in its vbare header. Every entry this client
/// writes is sealed with one, so the version stamped beside the payload always matches it.
/// A payload too short to hold a header reads as version 0, which no family has.
pub(crate) fn header_version(payload: &[u8]) -> u16 {
	match payload {
		[low, high, ..] => u16::from_le_bytes([*low, *high]),
		_ => 0,
	}
}

/// [`header_version`] for a payload this client read back, where a missing header is
/// corruption.
pub(crate) fn embedded_version(payload: &[u8]) -> Result<u16> {
	match header_version(payload) {
		0 => Err(corrupt!()),
		version => Ok(version),
	}
}

/// The `depot.fs_entry` version `versions` selects. A refusal means no version is readable
/// by every live storage worker, so nothing is admitted until the fleet converges.
pub(crate) fn select_entry_version(versions: &dyn WriteVersions) -> Result<u16> {
	versions
		.write_version(rivet_data::families::VersionFamily::DEPOT_FS_ENTRY)
		.map_err(|error| {
			tracing::warn!(
				?error,
				"no filesystem entry version is readable by every live storage worker; check for a deploy that skipped a release"
			);
			Error::Unavailable
		})
}

impl Journal for VolumeJournal {
	fn entry_version(&self) -> Result<u16> {
		select_entry_version(self.versions.as_ref())
	}

	fn spawn_cleanup(&self, cleanup: BoxFuture<'static, ()>) -> Result<()> {
		self.handle().spawn_cleanup(cleanup).map_err(Error::Log)
	}
	fn retry(&self, payload: Bytes, token: WriteToken) -> BoxFuture<'_, Result<WriteOutcome>> {
		Box::pin(async move {
			self.status()?;
			self.handle()
				.write(
					rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody::PayloadWrite(
						rivet_data::generated::depot_log_shard_v1::PayloadWrite {
							payload_family: rivet_data::families::VersionFamily::DEPOT_FS_ENTRY.0,
							payload_version: header_version(&payload),
							client_seq: 0,
							request_id: [0; 16],
							payload_digest: [0; 16],
							payload: payload.into(),
							codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec::None,
						},
					),
					Some(token),
				)
				.await
				.map_err(Error::Log)
		})
	}
	fn status(&self) -> Result<LogStatus> {
		if self.fenced.load(Ordering::Acquire) {
			return Err(self.fenced(crate::fences::FenceReason::JournalFenced, None));
		}
		if !(self.may_serve)() {
			return Err(self.fenced(crate::fences::FenceReason::DispatchRevoked, None));
		}
		let handle = self.handle();
		let view = handle.snapshot.borrow();
		// A journal exists only for a volume client that served, and a writer never reopens
		// itself, so any phase but appending means this client lost its volume or is ending.
		let not_serving = match &view.phase {
			// A serving client whose history the log shards permanently refuse at some entry
			// commits nothing behind it, so this filesystem ends and its owner reopens.
			VolumeClientPhase::Appending => view
				.history_blocked
				.map(|_| crate::fences::FenceReason::HistoryBlocked),
			VolumeClientPhase::Opening { .. } | VolumeClientPhase::Recovering(_) => {
				Some(crate::fences::FenceReason::VolumeClientNotServing)
			}
			VolumeClientPhase::Fenced { .. } => {
				Some(crate::fences::FenceReason::VolumeClientFenced)
			}
			VolumeClientPhase::Draining { .. }
			| VolumeClientPhase::Closing { .. }
			| VolumeClientPhase::Stopped => Some(crate::fences::FenceReason::VolumeClientClosed),
		};
		let not_serving = if handle.is_fenced() {
			Some(crate::fences::FenceReason::VolumeClientClosed)
		} else {
			not_serving
		};
		match not_serving {
			None => {}
			Some(reason) => {
				let phase = view.phase.clone();
				drop(view);
				return Err(self.fenced(reason, Some(&phase)));
			}
		}
		Ok(LogStatus {
			key: view.volume_log_key.as_deref().cloned().ok_or_else(|| {
				// The snapshot is still borrowed here, so this refusal only counts.
				crate::fences::count(crate::fences::FenceReason::VolumeUnresolved);
				Error::Fenced
			})?,
			cold: view.cold.clone(),
			committed: view.committed,
			next_seq: view.next_volume_seq,
			client_epoch: view.client.client_epoch,
			// 81 fixed bytes plus at most ten bytes for BARE's payload length.
			max_payload_bytes: (view.max_mutation_bytes as usize).saturating_sub(91),
			retained_bytes: view.memory_bytes as usize,
		})
	}
	fn recover(&self) -> BoxFuture<'_, Result<LogHistory>> {
		Box::pin(async move {
			if !(self.may_serve)() {
				return Err(self.fenced(crate::fences::FenceReason::DispatchRevoked, None));
			}
			let history = self.handle().history().await.map_err(Error::Log)?;
			self.status()?;
			Ok(history)
		})
	}

	fn read(&self, first: u64, max_bytes: usize) -> BoxFuture<'_, Result<LogHistory>> {
		Box::pin(async move {
			self.status()?;
			let page = self
				.handle()
				.read_committed(first, max_bytes.min(u32::MAX as usize) as u32)
				.await
				.map_err(Error::Log)?;
			self.status()?;
			Ok(page)
		})
	}

	fn admit(&self, payload: Bytes) -> BoxFuture<'_, Result<WriteOutcome>> {
		Box::pin(async move {
			self.status()?;
			let outcome = self
				.handle()
				.admit_payload(
					rivet_data::families::VersionFamily::DEPOT_FS_ENTRY,
					header_version(&payload),
					payload,
				)
				.await
				.map_err(Error::Log)?;
			Ok(outcome)
		})
	}

	fn barrier(&self) -> Result<CommitTicket> {
		self.status()?;
		self.handle().wait_for_commit().map_err(Error::Log)
	}
	fn confirm(&self) -> BoxFuture<'_, Result<u64>> {
		Box::pin(async move {
			self.status()?;
			self.handle().confirm().await.map_err(Error::Log)
		})
	}

	fn close(&self) -> BoxFuture<'_, Result<()>> {
		Box::pin(async move { self.handle().close().await.map_err(Error::Log) })
	}
	fn fence(&self) {
		self.fenced.store(true, Ordering::Release);
		self.handle().fence();
	}
	fn client_epoch(&self) -> Option<u64> {
		Some(self.handle().snapshot.borrow().client.client_epoch)
	}
	fn wait_progress(&self) -> BoxFuture<'_, Result<()>> {
		let mut snapshot = self.handle().snapshot.clone();
		// A cloned receiver keeps the handle's last seen version, which may be long past.
		// Mark the current state seen at the call so the wait ends only on a later change.
		snapshot.borrow_and_update();
		Box::pin(async move { snapshot.changed().await.map_err(|_| Error::Fenced) })
	}
}

/// The cold rows of one volume at a fixed floor position, as the claim holder reads them.
///
/// Every read names `through`, the sequence of the view it serves, and returns the rows of
/// some applied position from the floor through `through`. The claim holder overlays a layer
/// for every entry above the floor that the view includes, so for every row it reads here all
/// of those positions agree. `deadline_boot_ms` is the caller's deadline: a read can wait on
/// the maintainer until it has applied the floor, at most until then.
pub trait ColdSnapshot: Send + Sync {
	fn position(&self) -> &VolumeLogPosition;
	/// Reads one row.
	fn get(
		&self,
		key: RowKey,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Option<Row>>>;
	fn get_inode(
		&self,
		ino: u64,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Option<super::Inode>>> {
		Box::pin(async move {
			match self
				.get(RowKey::Inode(ino), through, deadline_boot_ms)
				.await?
			{
				None => Ok(None),
				Some(_) => Err(corrupt!()),
			}
		})
	}

	/// Returns the chunk rows of `ino` inside `ranges`, each `(first, count)` covering `first`
	/// through `first + count - 1`, that exist, in range then index order; a missing chunk is
	/// a hole. A store that can reads every range in one request. The default reads the
	/// chunks one at a time.
	fn chunks(
		&self,
		ino: u64,
		ranges: Vec<(u64, u32)>,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>> {
		Box::pin(async move {
			let mut rows = Vec::new();
			for (first, count) in ranges {
				for index in first..first.saturating_add(u64::from(count)) {
					let key = RowKey::Chunk(ino, index);
					if let Some(row) = self.get(key.clone(), through, deadline_boot_ms).await? {
						rows.push((key, row));
					}
				}
			}
			Ok(rows)
		})
	}

	/// Returns at most limit rows, in formal-key order, after an exclusive cursor.
	fn scan(
		&self,
		range: RowRange,
		after: Option<RowKey>,
		limit: usize,
		through: u64,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>>;
}

/// Rows an opening will read as soon as its snapshot is open, so a store that can reads them
/// with the open and answers both in one reply. Every opening reads the root inode, the
/// format and the first orphans. `path` names a file under the root; the open also returns
/// each step's dentry and inode and the final inode's `xattrs`. An empty prefetch still asks
/// for the root rows. Metadata only: a file's chunks are read afterwards, in
/// one request, so the open stays small while it holds its place in the store's order. `kv`
/// names KV inodes whose first rows the opening reads; see [`KvPrefetch`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenPrefetch {
	pub path: Vec<Vec<u8>>,
	pub xattrs: Vec<Vec<u8>>,
	pub kv: Vec<KvPrefetch>,
}

/// A KV inode's rows an opening reads first: `path` names the inode under the root, resolved
/// like [`OpenPrefetch::path`], and the open returns the inode's rows from `start`
/// (inclusive) to `end` (exclusive, unbounded when `None`), at most the first
/// [`depot_fs_core::wire::PREFETCH_KV_ROWS`] of them. The opening then serves its first reads
/// over that range, point and scan, from the reply, absent rows included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KvPrefetch {
	pub path: Vec<Vec<u8>>,
	pub start: Vec<u8>,
	pub end: Option<Vec<u8>>,
}

/// One row an open's prefetch found at the snapshot's position. An inode row carries the
/// sequence of its last change, which inode reads need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefetchedRow {
	pub key: RowKey,
	pub row: Row,
	pub change_seq: Option<u64>,
}

/// A snapshot a store opened, with the rows its prefetch found when the store served one.
/// `prefetched` is `None` when the store read nothing beside the position, so the opening
/// must not take any row for absent.
pub struct OpenedSnapshot {
	pub snapshot: Arc<dyn ColdSnapshot>,
	pub prefetched: Option<Vec<PrefetchedRow>>,
}

/// Opens go through the maintainer's request queue, so each carries a deadline that is never
/// later than its caller's. Nothing a store opens stays registered on the maintainer.
pub trait ColdStore: Send + Sync {
	/// Opens exactly `position`, or rejects it, never substituting a newer one, and reads
	/// `prefetch` there in the same request.
	fn open_prefetching(
		&self,
		volume: VolumeLogKey,
		position: VolumeLogPosition,
		prefetch: OpenPrefetch,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>>;

	/// Opens the newest position the store can serve, never below `floor`, reports it through
	/// [`ColdSnapshot::position`] and reads `prefetch` there in the same request. For a tablet
	/// maintainer that is its settled applied position, which can be far above cold, so the
	/// opening replays only what the maintainer has not applied.
	fn open_latest_prefetching(
		&self,
		volume: VolumeLogKey,
		floor: VolumeLogPosition,
		prefetch: OpenPrefetch,
		deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>>;

	/// The snapshot at `floor`, a committed position of the claim holder's own history, built
	/// without a request. The filesystem moves its snapshot to a newer floor when it drops the
	/// layers at or below it.
	fn claim_snapshot(
		&self,
		volume: VolumeLogKey,
		floor: VolumeLogPosition,
	) -> Arc<dyn ColdSnapshot>;

	/// The newest applied sequence a maintainer reply to this store reported, or 0 before any
	/// reply. A floor at or below it is one the maintainer already serves, so the misses after
	/// a move to it do not wait for an apply.
	fn observed_applied(&self) -> u64;
}
