//! An in-memory journal for the first write of a volume an open may create.
//!
//! A writer runs its first transaction against an empty filesystem over a [`CaptureJournal`],
//! which keeps each admitted entry instead of writing it. The entries become the creating
//! open's initial writes, so the volume's creation, its claim and that first commit share one
//! disk write. The commit waits for the open's answer: when the open created the volume with
//! those entries the commit succeeds with them durable; otherwise it fails before the writer
//! acts on it, and the writer runs again on the opened volume.

use crate::filesystem::{
	Error, Result,
	transport::{ColdSnapshot, ColdStore, Journal, LogStatus, OpenPrefetch, OpenedSnapshot},
};
use depot_fs_core::store::{Row, RowKey, RowRange};
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::{
	runner::{CommitTicket, LogHistory},
	types::{VolumeClientConfig, WriteError, WriteOutcome, WriteToken},
};
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use rivet_data::generated::{
	depot_log_shard_v1::{PayloadWrite, VolumeLogEntry, VolumeLogEntryBody},
	shared_depot_types_v1::VolumeLogPosition,
};
use rivet_topology::versions::WriteVersions;
use std::sync::Arc;
use tokio::sync::watch;

/// An empty volume's journal that keeps each admitted entry in memory instead of writing it.
///
/// A commit's barrier waits until the host decides, from the creating open's answer, whether
/// the captured entries became the volume's first writes. `Created` resolves every barrier,
/// so the command continues to its post-commit effects with its writes durable. `Existed`
/// fails them, so the command stops before any effect. No entry is admitted after the
/// decision. Forced-sync state: the filesystem calls its sync methods.
pub struct CaptureJournal {
	key: VolumeLogKey,
	/// Selects the entry version, as the volume client's journal would: the entries become
	/// the creating open's writes on the same log shards.
	versions: Arc<dyn WriteVersions>,
	admitted: Mutex<Vec<vbare::Bytes>>,
	/// Commit barriers taken so far. The host waits for the first one: the command then has
	/// its writes and waits for them to become durable.
	barriers: watch::Sender<u64>,
	/// Whether the open created the volume with the captured entries; `None` until it answers.
	created: watch::Sender<Option<bool>>,
}

impl CaptureJournal {
	pub fn new(key: VolumeLogKey, versions: Arc<dyn WriteVersions>) -> Self {
		Self {
			key,
			versions,
			admitted: Mutex::new(Vec::new()),
			barriers: watch::channel(0).0,
			created: watch::channel(None).0,
		}
	}

	/// The admitted entries in order, as the creating open's initial writes. Each carries
	/// the version its payload was encoded at.
	pub fn initial_writes(&self) -> Vec<VolumeLogEntryBody> {
		self.admitted
			.lock()
			.iter()
			.map(|payload| {
				VolumeLogEntryBody::PayloadWrite(PayloadWrite {
					payload_family: rivet_data::families::VersionFamily::DEPOT_FS_ENTRY.0,
					payload_version: crate::filesystem::transport::header_version(payload),
					client_seq: 0,
					request_id: [0; 16],
					payload_digest: [0; 16],
					payload: payload.clone(),
					codec: rivet_data::generated::depot_log_shard_v1::PayloadCodec::None,
				})
			})
			.collect()
	}

	/// Resolves once the command waits for a commit to become durable.
	pub async fn commit_requested(&self) {
		let mut barriers = self.barriers.subscribe();
		let _ = barriers.wait_for(|count| *count > 0).await;
	}

	/// Records the creating open's answer and releases or fails every waiting commit.
	pub fn decide(&self, created: bool) {
		self.created.send_replace(Some(created));
	}

	fn count(&self) -> u64 {
		self.admitted.lock().len() as u64
	}

	fn committed(&self) -> u64 {
		match *self.created.borrow() {
			Some(true) => self.count(),
			Some(false) | None => 0,
		}
	}
}

fn zero() -> VolumeLogPosition {
	VolumeLogPosition {
		volume_seq: 0,
		client_epoch: 0,
	}
}

impl Journal for CaptureJournal {
	fn entry_version(&self) -> Result<u16> {
		crate::filesystem::transport::select_entry_version(self.versions.as_ref())
	}

	fn status(&self) -> Result<LogStatus> {
		let count = self.count();
		Ok(LogStatus {
			key: self.key.clone(),
			cold: zero(),
			committed: self.committed(),
			next_seq: count + 1,
			client_epoch: 1,
			// The same bound the volume client applies, so the writes fit the open.
			max_payload_bytes: (VolumeClientConfig::default().max_append_bytes as usize)
				.saturating_sub(91 + 128),
			retained_bytes: 0,
		})
	}

	fn recover(&self) -> BoxFuture<'_, Result<LogHistory>> {
		Box::pin(async move {
			Ok(LogHistory {
				cold: zero(),
				committed: self.committed(),
				entries: Vec::new(),
				empty: true,
				complete: false,
			})
		})
	}

	/// A read of the command's own write must not take a commit barrier: the first barrier
	/// tells the host the command is done writing, and it would create the volume with only
	/// the entries admitted so far.
	fn reads_admitted(&self) -> bool {
		true
	}

	/// The command's own reads of rows it wrote find them here: the volume client never saw
	/// these entries, so they live only in this journal until the creating open writes them.
	fn read(&self, first: u64, max_bytes: usize) -> BoxFuture<'_, Result<LogHistory>> {
		Box::pin(async move {
			let mut bytes = 0usize;
			let entries = self
				.initial_writes()
				.into_iter()
				.zip(1u64..)
				.skip_while(|(_, volume_seq)| *volume_seq < first)
				.map(|(body, volume_seq)| VolumeLogEntry {
					written_at_unix_ms: 0,
					volume_seq,
					client_epoch: 1,
					body,
				})
				.take_while(|entry| {
					bytes =
						bytes.saturating_add(depot_types::log_shard::entry_bytes(entry) as usize);
					bytes <= max_bytes
				})
				.collect();
			Ok(LogHistory {
				cold: zero(),
				committed: self.committed(),
				entries,
				empty: false,
				complete: false,
			})
		})
	}

	fn admit(&self, payload: vbare::Bytes) -> BoxFuture<'_, Result<WriteOutcome>> {
		Box::pin(async move {
			if self.created.borrow().is_some() {
				crate::fences::count(crate::fences::FenceReason::CaptureCreated);
				return Err(Error::Fenced);
			}
			let mut admitted = self.admitted.lock();
			admitted.push(payload);
			let volume_seq = admitted.len() as u64;
			Ok(WriteOutcome {
				volume_seq,
				position: VolumeLogPosition {
					volume_seq,
					client_epoch: 1,
				},
				token: WriteToken {
					payload_digest: [0; 16],
					volume_incarnation: [0; 16],
					client_seq: volume_seq,
					request_id: [0; 16],
				},
				applied_earlier: false,
				committed: None,
			})
		})
	}

	fn barrier(&self) -> Result<CommitTicket> {
		let count = self.count();
		let mut created = self.created.subscribe();
		self.barriers.send_modify(|barriers| *barriers += 1);
		Ok(Box::pin(async move {
			let decided = created
				.wait_for(|created| created.is_some())
				.await
				.map(|created| *created);
			match decided {
				Ok(Some(true)) => Ok(count),
				Ok(Some(false) | None) | Err(_) => Err(WriteError::Fenced { token: None }),
			}
		}))
	}

	fn close(&self) -> BoxFuture<'_, Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn fence(&self) {}
}

/// The speculative volume has no cold rows, so nothing opens a cold snapshot of it.
pub struct NoColdStore;

impl ColdStore for NoColdStore {
	fn open_prefetching(
		&self,
		_volume: VolumeLogKey,
		_position: VolumeLogPosition,
		_prefetch: OpenPrefetch,
		_deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>> {
		Box::pin(async { Err(Error::Unavailable) })
	}

	fn open_latest_prefetching(
		&self,
		_volume: VolumeLogKey,
		_floor: VolumeLogPosition,
		_prefetch: OpenPrefetch,
		_deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<OpenedSnapshot>> {
		Box::pin(async { Err(Error::Unavailable) })
	}

	fn claim_snapshot(
		&self,
		_volume: VolumeLogKey,
		floor: VolumeLogPosition,
	) -> Arc<dyn ColdSnapshot> {
		Arc::new(NoColdSnapshot { floor })
	}

	fn observed_applied(&self) -> u64 {
		0
	}
}

/// A floor of the speculative volume. Its rows exist only in the capture's layers, so a read
/// that reaches it fails as unavailable instead of reporting a row as absent.
struct NoColdSnapshot {
	floor: VolumeLogPosition,
}

impl ColdSnapshot for NoColdSnapshot {
	fn position(&self) -> &VolumeLogPosition {
		&self.floor
	}

	fn get(
		&self,
		_key: RowKey,
		_through: u64,
		_deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Option<Row>>> {
		Box::pin(async { Err(Error::Unavailable) })
	}

	fn scan(
		&self,
		_range: RowRange,
		_after: Option<RowKey>,
		_limit: usize,
		_through: u64,
		_deadline_boot_ms: u64,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>> {
		Box::pin(async { Err(Error::Unavailable) })
	}
}


