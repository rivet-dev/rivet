//! Rolls an opened filesystem back to a checkpoint with ordinary writes on the same volume,
//! by its current owner, under the same incarnation.
//!
//! The rollback reads every metadata row of the checkpoint and of the head, plans the ops that
//! turn one into the other with [`depot_fs_core::rollback::plan`], and writes them as filesystem
//! transactions. A rollback whose ops fit in half of one transaction's byte cap commits in one
//! transaction, so readers see the head or the checkpoint and nothing in between. A larger one
//! commits in several transactions in plan order. The caller must keep every other writer out
//! until it returns, as the owner of a fenced volume does.
//!
//! The metadata both images hold while the plan is built is charged to one [`RollbackBudget`]
//! the process shares across every rollback. A rollback is refused as overloaded, before it
//! writes anything, when the budget is full at its start or a page of either image no longer
//! fits beside the rollbacks in progress; the caller may send it again once those finished. A
//! rollback whose images alone exceed the whole budget, or hold more than
//! `depot.filesystem.max_rollback_rows` rows each, is refused as too large, which no retry
//! clears. The two images are read at once, and the head's inodes are checked in batches sent
//! together, all within that budget.

use super::{Error, Filesystem, Result};
use depot_fs_core::{
	CHUNK_BYTES, FsChunkUpdate, FsOp, FsWrite,
	rollback::{ChunkMode, ChunkStep, Step},
	store::{Row, RowKey, RowRange, RowStore},
};
use futures_util::{StreamExt, TryStreamExt, future::BoxFuture};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Metadata rows one scan page reads.
const SCAN_PAGE_ROWS: usize = 512;

/// Inodes whose change sequence one read resolves.
const CHANGE_SEQ_BATCH: usize = 64;

/// Change sequence reads one rollback sends at once.
const CHANGE_SEQ_READS: usize = 8;

/// Encoded bytes one entry of a rollback holds at most, before the filesystem's own limit.
const MAX_ENTRY_BYTES: usize = 1024 * 1024;

/// The default of `depot.filesystem.max_rollback_bytes`, for a client built without the
/// configuration.
pub const DEFAULT_MAX_ROLLBACK_BYTES: usize = 512 * 1024 * 1024;

/// Bytes every map entry of a held image costs beyond its key's and row's own heap bytes: the
/// entry itself and the tree's per-entry share.
const ROW_OVERHEAD_BYTES: usize = 64;

/// The process-wide cap on metadata bytes that filesystem rollbacks hold at once, shared by
/// every rollback the process runs. Owned by the depot client and configured by
/// `depot.filesystem.max_rollback_bytes`.
#[derive(Debug)]
pub struct RollbackBudget {
	used: AtomicUsize,
	capacity: usize,
	near_cap: AtomicBool,
	_capacity_metric: IntGaugeGuard,
}

/// The bytes one rollback holds against its [`RollbackBudget`], released on drop.
pub struct RollbackCharge {
	budget: Arc<RollbackBudget>,
	bytes: usize,
	_metrics: Vec<IntGaugeGuard>,
}

impl RollbackBudget {
	pub fn new(capacity: usize) -> Arc<Self> {
		Arc::new(Self {
			used: AtomicUsize::new(0),
			capacity,
			near_cap: AtomicBool::new(false),
			_capacity_metric: crate::metrics::FS_ROLLBACK_CAPACITY.add_guard(capacity as i64),
		})
	}

	/// Bytes the rollbacks in progress hold.
	pub fn used(&self) -> usize {
		self.used.load(Ordering::Acquire)
	}

	/// Admits one rollback, refused as [`Error::Overloaded`] while the budget is full. The charge
	/// starts empty and grows with each page the rollback holds.
	pub fn admit(self: &Arc<Self>) -> Result<RollbackCharge> {
		if self.used() >= self.capacity {
			crate::metrics::FS_OVERLOADED
				.with_label_values(&["rollback_budget"])
				.inc();
			return Err(Error::Overloaded);
		}
		Ok(RollbackCharge {
			budget: self.clone(),
			bytes: 0,
			_metrics: Vec::new(),
		})
	}

	/// Warns once when usage crosses 90% of capacity and once when it falls back below.
	fn observe(&self, used: usize) {
		let near = used.saturating_mul(10) >= self.capacity.saturating_mul(9);
		if self.near_cap.swap(near, Ordering::AcqRel) != near {
			tracing::warn!(
				used,
				capacity = self.capacity,
				near,
				"filesystem rollback budget threshold crossed; concurrent rollbacks of large filesystems are refused at the cap, raise depot.filesystem.max_rollback_bytes if this persists"
			);
		}
	}
}

impl RollbackCharge {
	/// Adds `bytes` to the charge. A charge that would exceed the whole capacity on its own can
	/// never fit and is refused as [`Error::TooLarge`], which no retry clears; one that does not
	/// fit beside the other rollbacks in progress is refused as [`Error::Overloaded`].
	fn add(&mut self, bytes: usize) -> Result<()> {
		let budget = &self.budget;
		if self.bytes.saturating_add(bytes) > budget.capacity {
			return Err(Error::TooLarge);
		}
		let reserved = budget
			.used
			.try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
				used.checked_add(bytes)
					.filter(|next| *next <= budget.capacity)
			});
		let Ok(previous) = reserved else {
			crate::metrics::FS_OVERLOADED
				.with_label_values(&["rollback_budget"])
				.inc();
			return Err(Error::Overloaded);
		};
		budget.observe(previous.saturating_add(bytes));
		self.bytes += bytes;
		self._metrics
			.push(crate::metrics::FS_ROLLBACK_BYTES.add_guard(bytes as i64));
		Ok(())
	}
}

impl Drop for RollbackCharge {
	fn drop(&mut self) {
		let previous = self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
		self.budget.observe(previous.saturating_sub(self.bytes));
	}
}

/// The bytes one metadata row holds in a rollback's image: its key's and row's variable
/// parts, plus the per-entry share.
fn row_bytes(key: &RowKey, row: &Row) -> usize {
	let key_bytes = match key {
		RowKey::Dentry(_, name)
		| RowKey::Link(_, _, name)
		| RowKey::Xattr(_, name)
		| RowKey::Kv(_, name)
		| RowKey::KvRangeWhiteout(_, name) => name.len(),
		RowKey::Format | RowKey::Inode(_) | RowKey::Orphan(_) | RowKey::Chunk(_, _) => 0,
	};
	let row_bytes = match row {
		Row::Bytes(bytes) => bytes.len(),
		Row::KvRangeWhiteout(end) => end.as_ref().map_or(0, Vec::len),
		Row::Format(_) | Row::Inode(_) | Row::Dentry(_) | Row::Unit => 0,
	};
	std::mem::size_of::<(RowKey, Row)>() + ROW_OVERHEAD_BYTES + key_bytes + row_bytes
}

/// The volume as of one checkpoint, read without claiming it.
pub trait CheckpointImage: Send + Sync {
	/// The volume sequence the image shows.
	fn seq(&self) -> u64;

	/// Up to `limit` metadata rows after `after`, in key order. An empty page ends the scan.
	fn scan_metadata(
		&self,
		after: Option<RowKey>,
		limit: usize,
	) -> BoxFuture<'_, Result<Vec<(RowKey, Row)>>>;

	/// The content of chunks `first` through `first + count - 1` of file `ino`, one entry per
	/// chunk, with `None` for a hole. An entry may be shorter than a chunk; the rest is zeros.
	fn read_chunks(
		&self,
		ino: u64,
		first: u64,
		count: u64,
	) -> BoxFuture<'_, Result<Vec<Option<bytes::Bytes>>>>;
}

/// What a rollback wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackOutcome {
	/// The volume sequence of the last commit, or the head's when nothing differed.
	pub seq: u64,
	/// The transactions the rollback committed. More than one means readers could see the
	/// volume between the head and the checkpoint while it ran.
	pub transactions: usize,
}

impl Filesystem {
	/// Up to `limit` metadata rows of the committed head after `after`, in key order.
	pub async fn scan_metadata(
		&self,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<Vec<(RowKey, Row)>> {
		self.read_with(|v| v.scan(&RowRange::MetadataAll, after, limit, None))
			.await
	}

	/// Rolls the volume back to `checkpoint`; see the module documentation. The caller must be
	/// the volume's only writer for the whole call. The images it holds are charged to
	/// `budget`, the process's rollback budget.
	pub async fn rollback(
		self: &Arc<Self>,
		checkpoint: &dyn CheckpointImage,
		budget: &Arc<RollbackBudget>,
	) -> Result<RollbackOutcome> {
		let charge = parking_lot::Mutex::new(budget.admit()?);
		let max_rows = self.config.max_rollback_rows;
		crate::metrics::FS_ROLLBACK_ROWS_CAPACITY.set(i64::try_from(max_rows).unwrap_or(i64::MAX));
		let (at, head) = futures_util::try_join!(
			scan_all(&charge, max_rows, |after| {
				checkpoint.scan_metadata(after, SCAN_PAGE_ROWS)
			}),
			scan_all(&charge, max_rows, |after| {
				Box::pin(async move { self.scan_metadata(after.as_ref(), SCAN_PAGE_ROWS).await })
			}),
		)?;

		let candidates = depot_fs_core::rollback::content_candidates(&at, &head)?;
		let batches = futures_util::stream::iter(
			candidates
				.chunks(CHANGE_SEQ_BATCH)
				.map(<[u64]>::to_vec)
				.collect::<Vec<_>>(),
		)
		.map(|batch: Vec<u64>| async move {
			let seqs = self
				.read_with(|v| {
					batch
						.iter()
						.map(|&ino| v.inode(ino).map(|inode| inode.change_seq))
						.collect::<depot_fs_core::Result<Vec<_>>>()
				})
				.await?;
			Ok::<_, Error>(batch.into_iter().zip(seqs).collect::<Vec<_>>())
		})
		.buffer_unordered(CHANGE_SEQ_READS)
		.try_collect::<Vec<_>>()
		.await?;
		let changed = batches
			.into_iter()
			.flatten()
			.filter(|(_, seq)| *seq > checkpoint.seq())
			.map(|(ino, _)| ino)
			.collect::<BTreeSet<_>>();
		let plan = depot_fs_core::rollback::plan(&at, &head, &changed)?;
		drop((at, head, charge.into_inner()));

		if plan.steps.is_empty() {
			return Ok(RollbackOutcome {
				seq: self.state.lock().await.last_seq,
				transactions: 0,
			});
		}

		let entry_bytes = self
			.max_append_bytes()?
			.saturating_div(4)
			.min(MAX_ENTRY_BYTES);
		let transaction_bytes = self.max_transaction_bytes() / 2;
		let mut groups: Vec<Vec<Step>> = Vec::new();
		let mut group_bytes = 0usize;
		// A chunk step larger than one transaction allows is split, so every step fits in one.
		let chunks_per_step = (transaction_bytes / (CHUNK_BYTES * 2)).max(1) as u64;
		let steps = plan.steps.into_iter().flat_map(|step| match step {
			Step::Chunks(step) if step.count > chunks_per_step => {
				let mut split = Vec::new();
				let mut first = step.first;
				let end = step.first + step.count;
				while first < end {
					let count = (end - first).min(chunks_per_step);
					split.push(Step::Chunks(ChunkStep {
						first,
						count,
						..step.clone()
					}));
					first += count;
				}
				split
			}
			step => vec![step],
		});
		for step in steps {
			let bytes = step.max_bytes();
			match groups.last_mut() {
				Some(group) if group_bytes.saturating_add(bytes) <= transaction_bytes => {
					group.push(step);
					group_bytes += bytes;
				}
				Some(_) | None => {
					groups.push(vec![step]);
					group_bytes = bytes;
				}
			}
		}
		if groups.len() > 1 {
			tracing::info!(
				transactions = groups.len(),
				"filesystem rollback exceeds one transaction and commits in stages"
			);
		}

		let transactions = groups.len();
		let mut seq = 0;
		for group in groups {
			seq = self
				.rollback_transaction(checkpoint, group, entry_bytes)
				.await?;
		}
		Ok(RollbackOutcome { seq, transactions })
	}

	/// Writes `steps` in one transaction and returns its commit sequence.
	async fn rollback_transaction(
		self: &Arc<Self>,
		checkpoint: &dyn CheckpointImage,
		steps: Vec<Step>,
		entry_bytes: usize,
	) -> Result<u64> {
		let footprint: BTreeSet<u64> = steps.iter().flat_map(Step::inodes).collect();
		let mut txn = self.begin_txn(footprint).await?;
		let mut entry = Vec::new();
		let mut bytes = 0usize;
		for step in steps {
			let op = match step {
				Step::Op(op) => Some(op),
				Step::Chunks(step) => self.chunk_writes(checkpoint, &step).await?,
			};
			let Some(op) = op else {
				continue;
			};
			let op_bytes = depot_fs_core::rollback::op_bytes(&op);
			if !entry.is_empty() && bytes.saturating_add(op_bytes) > entry_bytes {
				txn.mutate_in(std::mem::take(&mut entry)).await?;
				bytes = 0;
			}
			entry.push(op);
			bytes += op_bytes;
		}
		if !entry.is_empty() {
			txn.mutate_in(entry).await?;
		}
		txn.commit(Vec::new()).await
	}

	/// The write that gives one chunk step the checkpoint's content, or `None` when every chunk
	/// it covers already holds it.
	async fn chunk_writes(
		&self,
		checkpoint: &dyn CheckpointImage,
		step: &ChunkStep,
	) -> Result<Option<FsOp>> {
		let ino = step.target.ino;
		let wanted = checkpoint.read_chunks(ino, step.first, step.count).await?;
		if wanted.len() != step.count as usize {
			return Err(corrupt!());
		}
		let start = step.first * CHUNK_BYTES as u64;
		let span = step.count as usize * CHUNK_BYTES;
		// The committed head, which the transaction's own pending writes are not part of. A
		// rewritten file's chunks below the checkpoint's size read the same under the head's
		// inode and under the postimage this rollback writes.
		let current = match &step.mode {
			ChunkMode::Copy => Vec::new(),
			ChunkMode::Compare { .. } => {
				let mut current = self.read(ino, start, span).await?;
				current.resize(span, 0);
				current
			}
		};
		let mut chunks = Vec::new();
		for (offset, stored) in wanted.into_iter().enumerate() {
			let index = step.first + offset as u64;
			let len = step.chunk_len(index);
			let mut bytes = stored.map(Vec::from).unwrap_or_default();
			if bytes.len() > CHUNK_BYTES {
				return Err(corrupt!());
			}
			bytes.resize(len, 0);
			let write = match &step.mode {
				ChunkMode::Copy => bytes.iter().any(|byte| *byte != 0),
				ChunkMode::Compare { force } => {
					let from = offset * CHUNK_BYTES;
					*force == Some(index) || bytes[..] != current[from..from + len]
				}
			};
			if write {
				chunks.push(FsChunkUpdate {
					index,
					bytes: bytes.into(),
				});
			}
		}
		Ok(match chunks.is_empty() {
			true => None,
			false => Some(FsOp::FsWrite(FsWrite {
				target: step.target.clone(),
				chunks,
			})),
		})
	}
}

/// Reads every page of one image's metadata, refusing an image above `max_rows` rows and a
/// page that does not fit in `charge`'s budget, which both images of one rollback share.
async fn scan_all<'a, F>(
	charge: &parking_lot::Mutex<RollbackCharge>,
	max_rows: usize,
	mut page: F,
) -> Result<BTreeMap<RowKey, Row>>
where
	F: FnMut(Option<RowKey>) -> BoxFuture<'a, Result<Vec<(RowKey, Row)>>>,
{
	let mut rows = BTreeMap::new();
	let mut after = None;
	loop {
		let found = page(after.clone()).await?;
		let Some((last, _)) = found.last() else {
			return Ok(rows);
		};
		if after.as_ref().is_some_and(|after| last <= after) {
			return Err(corrupt!());
		}
		after = Some(last.clone());
		let bytes = found.iter().map(|(key, row)| row_bytes(key, row)).sum();
		charge.lock().add(bytes)?;
		rows.extend(found);
		if rows.len() > max_rows {
			crate::metrics::FS_ROLLBACK_TOO_MANY_ROWS.inc();
			return Err(Error::TooLarge);
		}
	}
}
