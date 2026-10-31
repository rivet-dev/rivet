//! Pinned committed snapshots, range conflicts, and bounded group admission.

use super::{
	Config, Error, Filesystem, Result,
	cache::{Budget, Charge, Layer, View},
};
use depot_fs_core::{
	store::{Row, RowKey, RowRange, RowStore},
	*,
};
use std::{
	collections::VecDeque,
	ops::{Bound, RangeBounds},
	sync::Arc,
};
use tokio::sync::{Mutex, oneshot};
use vbare::Versioned;

/// Longest KV key a read or write accepts.
const MAX_KV_KEY_BYTES: usize = 2048;

/// Half-open byte range. Appending a zero byte converts an inclusive key bound
/// to the smallest exclusive bound, including keys containing arbitrary bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadRange {
	pub start: Vec<u8>,
	pub end: Option<Vec<u8>>,
}
impl RangeBounds<[u8]> for ReadRange {
	fn start_bound(&self) -> Bound<&[u8]> {
		Bound::Included(&self.start)
	}
	fn end_bound(&self) -> Bound<&[u8]> {
		self.end
			.as_deref()
			.map_or(Bound::Unbounded, Bound::Excluded)
	}
}
impl ReadRange {
	pub fn from_bounds(range: impl RangeBounds<[u8]>) -> Self {
		let start = match range.start_bound() {
			Bound::Unbounded => Vec::new(),
			Bound::Included(key) => key.to_vec(),
			Bound::Excluded(key) => {
				let mut key = key.to_vec();
				key.push(0);
				key
			}
		};
		let end = match range.end_bound() {
			Bound::Unbounded => None,
			Bound::Excluded(key) => Some(key.to_vec()),
			Bound::Included(key) => {
				let mut key = key.to_vec();
				key.push(0);
				Some(key)
			}
		};
		Self { start, end }
	}
	pub fn all() -> Self {
		Self {
			start: Vec::new(),
			end: None,
		}
	}
	pub fn key(key: &[u8]) -> Self {
		let mut end = key.to_vec();
		end.push(0);
		Self {
			start: key.to_vec(),
			end: Some(end),
		}
	}
	fn contains(&self, key: &[u8]) -> bool {
		key >= self.start.as_slice() && self.end.as_ref().is_none_or(|end| key < end.as_slice())
	}
	fn overlaps(&self, other: &Self) -> bool {
		self.end.as_ref().is_none_or(|end| &other.start < end)
			&& other.end.as_ref().is_none_or(|end| &self.start < end)
	}
	fn valid(&self) -> bool {
		self.end.as_ref().is_none_or(|end| &self.start <= end)
	}
}
#[derive(Clone)]
pub(super) struct WriteRange {
	ino: u64,
	range: ReadRange,
}
fn write_ranges(entry: &FsEntry) -> Vec<WriteRange> {
	entry
		.ops
		.iter()
		.filter_map(|op| match op {
			FsOp::FsKvPut(x) => Some(WriteRange {
				ino: x.ino,
				range: ReadRange::key(&x.key),
			}),
			FsOp::FsKvDelete(x) => Some(WriteRange {
				ino: x.ino,
				range: ReadRange::key(&x.key),
			}),
			FsOp::FsKvDeleteRange(x) => Some(WriteRange {
				ino: x.ino,
				range: ReadRange {
					start: x.start.to_vec(),
					end: x.end.clone().map(Into::into),
				},
			}),
			FsOp::FsReclaim(x) => Some(WriteRange {
				ino: x.ino,
				range: ReadRange::all(),
			}),
			_ => None,
		})
		.collect()
}
pub(super) struct Change {
	seq: u64,
	at_boot_ms: u64,
	writes: Vec<WriteRange>,
	bytes: usize,
	_charge: Charge,
}
impl Change {
	pub(crate) fn new(
		seq: u64,
		at_boot_ms: u64,
		entry: &FsEntry,
		budget: &Arc<Budget>,
	) -> Result<Self> {
		Self::from_writes(seq, at_boot_ms, write_ranges(entry), budget)
	}
	pub(crate) fn footprint(
		seq: u64,
		at_boot_ms: u64,
		inodes: &std::collections::BTreeSet<u64>,
		budget: &Arc<Budget>,
	) -> Result<Self> {
		Self::from_writes(
			seq,
			at_boot_ms,
			inodes
				.iter()
				.map(|ino| WriteRange {
					ino: *ino,
					range: ReadRange::all(),
				})
				.collect(),
			budget,
		)
	}
	fn from_writes(
		seq: u64,
		at_boot_ms: u64,
		writes: Vec<WriteRange>,
		budget: &Arc<Budget>,
	) -> Result<Self> {
		let bytes = writes
			.iter()
			.map(|w| 96 + w.range.start.len() + w.range.end.as_ref().map_or(0, Vec::len))
			.sum::<usize>()
			+ 64;
		Ok(Self {
			seq,
			at_boot_ms,
			writes,
			bytes,
			_charge: budget.reserve(bytes)?,
		})
	}
}
#[derive(Default)]
pub(super) struct ConflictWindow {
	floor: u64,
	bytes: usize,
	changes: VecDeque<Change>,
}
impl ConflictWindow {
	pub(crate) fn push(&mut self, change: Change, config: &Config) {
		let now = change.at_boot_ms;
		self.bytes += change.bytes;
		self.changes.push_back(change);
		while self.bytes > config.max_conflict_window_bytes
			|| self
				.changes
				.front()
				.is_some_and(|c| now.saturating_sub(c.at_boot_ms) >= config.max_snapshot_age_ms)
		{
			let Some(old) = self.changes.pop_front() else {
				break;
			};
			self.bytes -= old.bytes;
			self.floor = self.floor.max(old.seq);
		}
	}
	fn validate(&self, tx: &Transaction, now: u64) -> Result<()> {
		if tx
			.pin
			.invalidated
			.load(std::sync::atomic::Ordering::Acquire)
			|| now >= tx.pin.expires_boot_ms
		{
			return Err(Error::TooOld);
		}
		if tx.reads.is_empty() {
			return Ok(());
		}
		if tx.pin.seq < self.floor {
			return Err(Error::TooOld);
		}
		if self
			.changes
			.iter()
			.filter(|c| c.seq > tx.pin.seq)
			.any(|c| tx.conflicts(&c.writes))
		{
			return Err(Error::Conflict);
		}
		Ok(())
	}
}
pub(super) struct Pin {
	pub invalidated: std::sync::atomic::AtomicBool,
	pub seq: u64,
	pub expires_boot_ms: u64,
	pub view: Mutex<Option<View>>,
}
pub struct Transaction {
	fs: Arc<Filesystem>,
	ino: u64,
	pin: Arc<Pin>,
	reads: Vec<ReadRange>,
	alarm_read: bool,
	ops: Vec<FsOp>,
	charges: Vec<Charge>,
	bytes: usize,
}
pub(super) struct Pending {
	tx: Transaction,
	deadline: u64,
	reply: oneshot::Sender<Result<u64>>,
}
#[derive(Default)]
pub(super) struct Queue {
	pending: VecDeque<Pending>,
	driving: bool,
}

impl Filesystem {
	pub async fn begin(self: &Arc<Self>, ino: u64) -> Result<Transaction> {
		let deadline = self.deadline()?;
		{
			let result = async {
				let mut state = self.state.lock().await;
				// The transaction's view stops at the committed position.
				let status = self.refresh_committed(&mut state, deadline).await?;
				if state.pins.len() >= self.config.max_snapshots {
					crate::metrics::FS_OVERLOADED
						.with_label_values(&["snapshots"])
						.inc();
					return Err(Error::Overloaded);
				}
				// A new transaction cannot observe an admitted but unacknowledged entry.
				let seq = status.committed.min(state.last_seq);
				if seq < state.cold.snapshot.position().volume_seq {
					return Err(Error::Lagging);
				}
				let mut view = self.view(&state, seq);
				let inode = match self
					.evaluate(&mut view, deadline, |v| store::inode(v, ino, None))
					.await
				{
					// The log trimmed an entry this view needs while the filesystem was idle.
					// The same sequence reads on over a newer cold snapshot, without the reopen
					// a retryable error would cost.
					Err(Error::TooOld) => {
						drop(state);
						let inode = self
							.rebase_trimmed(
								&mut view,
								deadline,
								|v| store::inode(v, ino, None),
								Err(Error::TooOld),
							)
							.await?;
						state = self.state.lock().await;
						inode
					}
					result => result?,
				};
				if inode.kind != FsKind::Kv {
					return Err(depot_fs_core::Error::Invalid.into());
				}
				let pin = Arc::new(Pin {
					invalidated: std::sync::atomic::AtomicBool::new(false),
					seq,
					expires_boot_ms: self
						.now()?
						.boot_ms
						.saturating_add(self.config.max_snapshot_age_ms),
					view: Mutex::new(Some(view)),
				});
				state.pins.push(Arc::downgrade(&pin));
				Ok(Transaction {
					fs: self.clone(),
					ino,
					pin,
					reads: Vec::new(),
					alarm_read: false,
					ops: Vec::new(),
					charges: Vec::new(),
					bytes: 0,
				})
			}
			.await;
			result
		}
	}

	async fn commit_transaction(self: &Arc<Self>, tx: Transaction) -> Result<u64> {
		let deadline = self.deadline()?;
		self.recover_before(deadline).await?;
		let (reply, mut receive) = oneshot::channel();
		{
			let mut queue = self.queue.lock();
			if queue.pending.len() >= self.config.max_pending_commits {
				crate::metrics::FS_OVERLOADED
					.with_label_values(&["pending_commits"])
					.inc();
				return Err(Error::Overloaded);
			}
			queue.pending.push_back(Pending {
				tx,
				deadline,
				reply,
			});
		}
		// Time spent waiting while another caller drove a commit group, observed once per commit.
		let mut queue_wait = QueueWait::default();
		loop {
			// Register before checking the queue to avoid losing the previous
			// driver's completion between deciding to wait and polling Notify.
			let wake = self.queue_wake.notified();
			tokio::pin!(wake);
			wake.as_mut().enable();
			match receive.try_recv() {
				Ok(result) => {
					queue_wait.finish();
					return result;
				}
				Err(oneshot::error::TryRecvError::Closed) => {
					return Err(self.fenced(crate::fences::FenceReason::CommitDriverAbandoned));
				}
				Err(_) => {}
			}
			let drive = {
				let mut queue = self.queue.lock();
				if queue.driving {
					false
				} else {
					queue.driving = true;
					true
				}
			};
			if drive {
				queue_wait.finish();
				self.drive_group().await;
			} else {
				#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
				let started = std::time::Instant::now();
				let woke = rivet_env::timeout_at(self.clock.as_ref(), deadline, wake).await;
				queue_wait.add(started.elapsed());
				woke.map_err(|_| Error::DeadlineExceeded)?;
			}
		}
	}
	async fn drive_group(&self) {
		struct Driver<'a> {
			fs: &'a Filesystem,
			finished: bool,
		}
		impl Drop for Driver<'_> {
			fn drop(&mut self) {
				let mut queue = self.fs.queue.lock();
				queue.driving = false;
				if !self.finished {
					self.fs
						.fence_with(crate::fences::FenceReason::CommitDriverAbandoned);
					for pending in queue.pending.drain(..) {
						let _ = pending.reply.send(Err(Error::Fenced));
					}
				}
				self.fs.queue_wake.notify_waiters();
			}
		}
		let mut driver = Driver {
			fs: self,
			finished: false,
		};
		let pending = {
			let mut queue = self.queue.lock();
			std::mem::take(&mut queue.pending)
		};
		let deadline = pending.iter().map(|p| p.deadline).min().unwrap_or(0);
		let mut accepted = Vec::new();
		let result = self.prepare_group(pending, &mut accepted).await;
		let result = self.pinned_result(result, deadline).await;
		for pending in accepted {
			if result.is_ok() && !pending.tx.ops.is_empty() {
				crate::metrics::KV_COMMITTED.inc();
			}
			let _ = pending.reply.send(result.clone());
		}
		driver.finished = true;
	}
	async fn prepare_group(
		&self,
		mut pending: VecDeque<Pending>,
		accepted: &mut Vec<Pending>,
	) -> Result<u64> {
		let deadline = pending
			.iter()
			.map(|p| p.deadline)
			.min()
			.unwrap_or(self.deadline()?);
		let _write = loop {
			let wake = self.transaction_wake.notified();
			tokio::pin!(wake);
			wake.as_mut().enable();
			let write =
				rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
			let state = self.state.lock().await;
			if !pending.iter().any(|p| {
				state
					.transactions
					.values()
					.any(|t| t.footprint.contains(&p.tx.ino))
			}) {
				break write;
			}
			drop(state);
			drop(write);
			rivet_env::timeout_at(self.clock.as_ref(), deadline, wake)
				.await
				.map_err(|_| Error::DeadlineExceeded)?;
		};
		let mut state = self.state.lock().await;
		// Send every dequeued caller its exact error if refreshing the snapshot
		// fails; no caller is silently stranded outside the queue.
		let mut status = match self.refresh(&mut state, deadline).await {
			Ok(status) => status,
			Err(error) => {
				drop(state);
				let error = self
					.pinned_result::<()>(Err(error), deadline)
					.await
					.unwrap_err();
				for p in pending {
					let _ = p.reply.send(Err(error.clone()));
				}
				return Err(error);
			}
		};
		// The batch is sized at the version admission will encode it at.
		let entry_version = match self.journal.entry_version() {
			Ok(version) => version,
			Err(error) => {
				drop(state);
				for p in pending {
					let _ = p.reply.send(Err(error.clone()));
				}
				return Err(error);
			}
		};
		let mut view = self.view(&state, state.last_seq);
		let mut entry = FsEntry {
			txn: None,
			ops: Vec::new(),
		};
		let mut writes = Vec::new();
		let mut rebased_without_evidence = false;
		while let Some(p) = pending.pop_front() {
			if p.reply.is_closed() {
				continue;
			}
			let validation = state
				.conflicts
				.validate(&p.tx, self.now()?.boot_ms)
				.and_then(|_| {
					if p.tx.conflicts(&writes) {
						Err(Error::Conflict)
					} else {
						Ok(())
					}
				});
			if let Err(error) = validation {
				let _ = p.reply.send(Err(error));
				continue;
			}
			let mut inode = self
				.evaluate(&mut view, deadline, |v| store::inode(v, p.tx.ino, None))
				.await;

			// The log offloaded and trimmed an entry the group's view reads while this
			// filesystem had not learned the newer cold position. Nothing in the group was
			// admitted yet and the write gate keeps `last_seq` fixed, so the view rebuilt at
			// the same sequence over the newer cold snapshot reads the same state. Cold can
			// move again before the rebuilt view reads, so this repeats while each rebuild
			// lands on a newer cold position, as `Filesystem::rebase_trimmed` does; one
			// rebuild without that evidence is allowed per group. The deadline bounds it.
			while matches!(inode, Err(Error::TooOld)) {
				let cold_before = view.cold.snapshot.position().volume_seq;
				let seq = state.last_seq;
				drop(state);
				let rebased = self.rebase_view(&mut view, seq, deadline).await;
				state = self.state.lock().await;
				if !matches!(rebased, Ok(true)) || state.last_seq != seq {
					break;
				}
				if view.cold.snapshot.position().volume_seq <= cold_before {
					if rebased_without_evidence {
						break;
					}
					rebased_without_evidence = true;
				}
				match self.refresh(&mut state, deadline).await {
					Ok(refreshed) => {
						status = refreshed;
						inode = self
							.evaluate(&mut view, deadline, |v| store::inode(v, p.tx.ino, None))
							.await;
					}
					Err(error) => {
						inode = Err(error);
						break;
					}
				}
			}

			if !matches!(
				inode,
				Ok(FsInode {
					kind: FsKind::Kv,
					..
				})
			) {
				let error = inode.err().unwrap_or(depot_fs_core::Error::Invalid.into());
				let _ = p.reply.send(Err(error));
				continue;
			}
			let old_len = entry.ops.len();
			entry.ops.extend(p.tx.ops.iter().cloned());
			let bytes = rivet_data::versioned::DepotFsEntryData::serialize_with_embedded_version(
				&entry,
				entry_version,
			)
			.map_err(|_| corrupt!())?
			.len();
			if bytes > status.max_payload_bytes || bytes > self.config.max_operation_bytes {
				entry.ops.truncate(old_len);
				if accepted.is_empty() {
					let _ = p.reply.send(Err(Error::TooLarge));
					continue;
				}
				pending.push_front(p);
				let mut queue = self.queue.lock();
				pending.append(&mut queue.pending);
				queue.pending = pending;
				break;
			}
			writes.extend(write_ranges(&FsEntry {
				txn: None,
				ops: p.tx.ops.clone(),
			}));
			accepted.push(p);
		}
		if accepted.is_empty() {
			return Ok(state.last_seq);
		}
		if entry.ops.is_empty() {
			let seq = state.last_seq;
			return Ok(seq);
		}
		let receipt = self.admit(&mut state, entry, status, deadline).await?;
		drop(state);
		self.finish_write(receipt, deadline)
			.await
			.map(|committed| committed.volume_seq)
	}
}
impl Transaction {
	/// Reads volume-wide alarm state under this transaction's pinned snapshot.
	pub async fn alarm_summary(
		&mut self,
	) -> Result<Option<rivet_data::generated::depot_alarm_v1::AlarmValue>> {
		use depot_types::alarm::COLD_ALARM_KEY;
		use rivet_data::versioned::DepotVolumeAlarmData;
		let deadline = self.fs.deadline()?.min(self.pin.expires_boot_ms);
		self.fs.recover_before(deadline).await?;
		self.check()?;
		self.alarm_read = true;
		let staged = self.ops.iter().rev().find_map(|op| match op {
			FsOp::FsKvPut(v) if v.ino == 0 && v.key.as_ref() == COLD_ALARM_KEY => {
				Some(v.value.clone())
			}
			_ => None,
		});
		let bytes = if let Some(bytes) = staged {
			Some(bytes)
		} else {
			let mut pin = self.pin.view.lock().await;
			let view = pin.as_mut().ok_or(Error::TooOld)?;
			let value = self
				.fs
				.evaluate(view, deadline, |v| {
					v.get(&RowKey::Kv(0, COLD_ALARM_KEY.to_vec()), None)
				})
				.await;
			drop(pin);
			match self.fs.pinned_result(value, deadline).await? {
				Some(Row::Bytes(bytes)) => Some(bytes),
				None => None,
				_ => return Err(corrupt!()),
			}
		};
		self.check()?;
		bytes
			.map(|bytes| {
				DepotVolumeAlarmData::deserialize_with_embedded_version(&bytes)
					.map_err(|_| corrupt!())
			})
			.transpose()
	}

	fn write_alarm(
		&mut self,
		pending: Option<rivet_data::generated::shared_depot_alarms_v1::AlarmSet>,
	) -> Result<()> {
		use depot_types::alarm::COLD_ALARM_KEY;
		use rivet_data::{generated::depot_alarm_v1::AlarmValue, versioned::DepotVolumeAlarmData};
		self.check()?;
		let value = DepotVolumeAlarmData::seal_to_bytes(
			&AlarmValue { pending },
			DepotVolumeAlarmData::LATEST,
		)
		.map_err(|_| corrupt!())?;
		self.charge(96 + COLD_ALARM_KEY.len() + value.len())?;
		self.ops.push(FsOp::FsKvPut(FsKvPut {
			ino: 0,
			key: COLD_ALARM_KEY.to_vec().into(),
			value,
		}));
		Ok(())
	}
	pub fn set_alarm(&mut self, deadline_unix_ms: i64, arm_id: [u8; 16]) -> Result<()> {
		self.write_alarm(Some(
			rivet_data::generated::shared_depot_alarms_v1::AlarmSet {
				arm_id,
				deadline_unix_ms,
			},
		))
	}
	pub fn clear_alarm(&mut self) -> Result<()> {
		self.write_alarm(None)
	}
	/// The read conflict makes consuming this revision atomic with every other write.
	pub async fn ack_alarm(
		&mut self,
		expected: rivet_data::generated::shared_depot_alarms_v1::AlarmRevision,
		next: Option<rivet_data::generated::shared_depot_alarms_v1::AlarmSet>,
	) -> Result<()> {
		let status = self.fs.status()?;
		let current = self.alarm_summary().await?.and_then(|value| value.pending);
		if status.key.0.volume_incarnation != expected.volume_incarnation
			|| current.as_ref().is_none_or(|v| v.arm_id != expected.arm_id)
		{
			return Err(Error::Conflict);
		}
		self.write_alarm(next)
	}
	pub fn read_version(&self) -> u64 {
		self.pin.seq
	}
	fn check(&self) -> Result<()> {
		self.fs.status()?;
		if self
			.pin
			.invalidated
			.load(std::sync::atomic::Ordering::Acquire)
			|| self.fs.now()?.boot_ms >= self.pin.expires_boot_ms
		{
			return Err(Error::TooOld);
		}
		Ok(())
	}
	fn key(&self, key: &[u8]) -> Result<()> {
		if key.len() > MAX_KV_KEY_BYTES {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		Ok(())
	}
	fn charge(&mut self, bytes: usize) -> Result<()> {
		let next = self
			.bytes
			.checked_add(bytes)
			.filter(|n| *n <= self.fs.config.max_operation_bytes)
			.ok_or(Error::TooLarge)?;
		self.charges.push(self.fs.budget.reserve(bytes)?);
		self.bytes = next;
		Ok(())
	}
	fn conflicts(&self, writes: &[WriteRange]) -> bool {
		writes.iter().any(|w| {
			(w.ino == self.ino && self.reads.iter().any(|r| r.overlaps(&w.range)))
				|| (self.alarm_read && w.ino == 0)
		})
	}
	pub fn add_read_conflict_range(&mut self, range: impl RangeBounds<[u8]>) -> Result<()> {
		let range = ReadRange::from_bounds(range);
		self.check()?;
		if !range.valid() {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		self.charge(64 + range.start.len() + range.end.as_ref().map_or(0, Vec::len))?;
		self.reads.push(range);
		Ok(())
	}
	pub async fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		self.add_read_conflict_range(ReadRange::key(key))?;
		self.get_untracked(key).await
	}
	pub async fn get_untracked(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		let deadline = self.fs.deadline()?.min(self.pin.expires_boot_ms);
		self.fs.recover_before(deadline).await?;
		self.check()?;
		self.key(key)?;
		for op in self.ops.iter().rev() {
			match op {
				FsOp::FsKvPut(x) if x.ino == self.ino && x.key == key => {
					return Ok(Some(x.value.to_vec()));
				}
				FsOp::FsKvDelete(x) if x.ino == self.ino && x.key == key => return Ok(None),
				FsOp::FsKvDeleteRange(x)
					if x.ino == self.ino
						&& (ReadRange {
							start: x.start.to_vec(),
							end: x.end.clone().map(Into::into),
						})
						.contains(key) =>
				{
					return Ok(None);
				}
				_ => {}
			}
		}
		let mut pin = self.pin.view.lock().await;
		let view = pin.as_mut().ok_or(Error::TooOld)?;
		let value = self
			.fs
			.evaluate_rebasing(view, deadline, |v| {
				v.get(&RowKey::Kv(self.ino, key.to_vec()), None)
			})
			.await;
		drop(pin);
		let value = self.fs.pinned_result(value, deadline).await?;
		self.check()?;
		match value {
			Some(Row::Bytes(value)) => Ok(Some(Vec::from(value))),
			None => Ok(None),
			_ => Err(corrupt!()),
		}
	}
	pub async fn scan(
		&mut self,
		range: impl RangeBounds<[u8]>,
		limit: Option<usize>,
		reverse: bool,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let deadline = self.fs.deadline()?.min(self.pin.expires_boot_ms);
		self.fs.recover_before(deadline).await?;
		let range = ReadRange::from_bounds(range);
		self.check()?;
		if !range.valid() {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		if limit == Some(0) {
			return Ok(Vec::new());
		}
		let mut pin = self.pin.view.lock().await;
		let pinned = pin.as_mut().ok_or(Error::TooOld)?;
		let entry = FsEntry {
			txn: None,
			ops: self.ops.clone(),
		};
		let batch = apply::plan(
			self.pin.seq.checked_add(1).ok_or_else(|| corrupt!())?,
			&entry,
		)?;
		let mut view = pinned.fork();
		view.seq = self.pin.seq + 1;
		view.layers
			.push(Layer::new(view.seq, batch, &self.fs.budget)?);
		drop(pin);
		let row_range = RowRange::Kv {
			ino: self.ino,
			start: range.start.clone(),
			end: range.end.clone(),
		};
		let mut retained = VecDeque::new();
		let mut retained_bytes = 0usize;
		let mut cursor = None;
		loop {
			let remaining = if reverse {
				256
			} else {
				limit
					.unwrap_or(usize::MAX)
					.saturating_sub(retained.len())
					.min(256)
			};
			if remaining == 0 {
				break;
			}
			let page = self
				.fs
				.evaluate(&mut view, deadline, |v| {
					v.scan(&row_range, cursor.as_ref(), remaining, None)
				})
				.await;
			let page = self.fs.pinned_result(page, deadline).await?;
			let end = page.len() < remaining;
			cursor = page.last().map(|(key, _)| key.clone());
			view.forget_fetched();
			for (key, value) in page {
				if reverse && limit.is_some_and(|limit| retained.len() == limit) {
					let (_, _, old_bytes, _charge) =
						retained.pop_front().ok_or_else(|| corrupt!())?;
					retained_bytes -= old_bytes;
				}
				let bytes = super::cache::row_bytes(&key, &Some(value.clone()));
				retained_bytes = retained_bytes
					.checked_add(bytes)
					.filter(|n| *n <= self.fs.config.max_operation_bytes)
					.ok_or(Error::TooLarge)?;
				let charge = self.fs.budget.reserve(bytes)?;
				match (key, value) {
					(RowKey::Kv(_, key), Row::Bytes(value)) => {
						retained.push_back((key, Vec::from(value), bytes, charge))
					}
					_ => return Err(corrupt!()),
				}
			}
			if end {
				break;
			}
		}
		self.check()?;
		let mut rows: Vec<_> = retained
			.into_iter()
			.map(|(key, value, _, _)| (key, value))
			.collect();
		if reverse {
			rows.reverse();
		}

		let mut observed = range;
		if limit.is_some_and(|limit| rows.len() == limit)
			&& let Some((last, _)) = rows.last()
		{
			if reverse {
				observed.start = last.clone();
			} else {
				observed.end = ReadRange::key(last).end;
			}
		}
		self.add_read_conflict_range(observed)?;
		Ok(rows)
	}
	pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
		self.check()?;
		self.key(key)?;
		if value.len() > self.fs.config.max_value_bytes {
			return Err(Error::TooLarge);
		}
		self.charge(96 + key.len() + value.len())?;
		self.ops.push(FsOp::FsKvPut(FsKvPut {
			ino: self.ino,
			key: key.to_vec().into(),
			value: value.to_vec().into(),
		}));
		Ok(())
	}
	pub fn delete(&mut self, key: &[u8]) -> Result<()> {
		self.check()?;
		self.key(key)?;
		self.charge(96 + key.len())?;
		self.ops.push(FsOp::FsKvDelete(FsKvDelete {
			ino: self.ino,
			key: key.to_vec().into(),
		}));
		Ok(())
	}
	pub fn delete_range(&mut self, range: impl RangeBounds<[u8]>) -> Result<()> {
		let range = ReadRange::from_bounds(range);
		self.check()?;
		if !range.valid() {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		self.charge(96 + range.start.len() + range.end.as_ref().map_or(0, Vec::len))?;
		self.ops.push(FsOp::FsKvDeleteRange(FsKvDeleteRange {
			ino: self.ino,
			start: range.start.into(),
			end: range.end.map(Into::into),
		}));
		Ok(())
	}
	pub async fn commit(self) -> Result<u64> {
		let fs = self.fs.clone();
		fs.commit_transaction(self).await
	}
}

/// Rows one page of a pinless KV scan asks the store for, the page size the store serves.
const KV_SCAN_PAGE_ROWS: usize = 256;

impl Filesystem {
	/// Reads one KV row at the committed head without pinning a snapshot, so it is not bounded
	/// by the snapshot table and sees every entry this filesystem has committed, applied or not,
	/// as well as what the open's prefetch returned. A cold row costs one maintainer request
	/// and is then cached. For a read whose conflicts a later commit must detect, use
	/// [`Filesystem::begin`] instead.
	pub async fn kv_get(&self, ino: u64, key: &[u8]) -> Result<Option<Vec<u8>>> {
		if key.len() > MAX_KV_KEY_BYTES {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		let row_key = RowKey::Kv(ino, key.to_vec());
		let value = self.read_with(|view| view.get(&row_key, None)).await?;
		match value {
			Some(Row::Bytes(value)) => Ok(Some(Vec::from(value))),
			None => Ok(None),
			Some(Row::Format(_) | Row::Inode(_) | Row::Dentry(_) | Row::Unit) => Err(corrupt!()),
		}
	}

	/// Reads up to `limit` KV rows of `ino` in `range`, in key order, at the committed head
	/// without pinning a snapshot; see [`Filesystem::kv_get`]. Forward only: the store scans
	/// forward, so a caller that needs the last rows below a key scans a bounded range below
	/// it. The result is bounded by the operation byte cap.
	pub async fn kv_scan(
		&self,
		ino: u64,
		range: impl RangeBounds<[u8]>,
		limit: usize,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let range = ReadRange::from_bounds(range);
		if !range.valid() {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		if limit == 0 {
			return Ok(Vec::new());
		}
		let row_range = RowRange::Kv {
			ino,
			start: range.start,
			end: range.end,
		};
		// The view keeps each page it loaded, so a page the closure asks for again after a
		// miss is already there and the loop picks up where it was.
		self.read_with(|view| {
			let mut rows = Vec::new();
			let mut cursor: Option<RowKey> = None;
			loop {
				let remaining = limit.saturating_sub(rows.len()).min(KV_SCAN_PAGE_ROWS);
				if remaining == 0 {
					break;
				}
				let page = view.scan(&row_range, cursor.as_ref(), remaining, None)?;
				let end = page.len() < remaining;
				cursor = page.last().map(|(key, _)| key.clone());
				for (key, value) in page {
					let (RowKey::Kv(_, key), Row::Bytes(value)) = (key, value) else {
						return Err(depot_fs_core::Error::Corrupt);
					};
					rows.push((key, Vec::from(value)));
				}
				if end {
					break;
				}
			}
			Ok(rows)
		})
		.await
	}
}

/// A commit's wait for another caller's commit group, observed once by `finish` as
/// `depot_filesystem_commit_queue_seconds`. A commit that drove its own group at once records
/// zero.
#[derive(Default)]
struct QueueWait {
	waited: std::time::Duration,
	observed: bool,
}

impl QueueWait {
	fn add(&mut self, elapsed: std::time::Duration) {
		self.waited += elapsed;
	}

	fn finish(&mut self) {
		if self.observed {
			return;
		}
		self.observed = true;
		crate::metrics::FS_COMMIT_QUEUE_SECONDS.observe(self.waited.as_secs_f64());
	}
}
