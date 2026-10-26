//! Claim-bound filesystem transactions. Footprint reservations also cover the
//! interval before the first pending entry and after cancellation until abort.
use super::{
	Error, Filesystem, Inode, Result,
	cache::{Charge, Layer, View},
	client::State,
	file_batch::{self, FileEdit},
};
use depot_fs_core::{txn, *};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::{collections::BTreeSet, sync::Arc};

/// Delay before background cleanup tries an undecided transaction abort again. Each try waits
/// up to its own request deadline, so this only spaces tries out.
const ABORT_RETRY_MS: u64 = 1_000;

/// A local reservation of one transaction; a reused staged-write ID is a different owner.
#[derive(Clone, Copy, PartialEq, Eq)]
struct TransactionOwner {
	reservation: u64,
}

pub(super) struct Pending {
	owner: TransactionOwner,
	pub footprint: BTreeSet<u64>,
	pub layers: Vec<Arc<Layer>>,
	pub bytes: usize,
	pub expires: u64,
	pub abort_reserve: Option<Charge>,
	pub _expiry: tokio::sync::oneshot::Sender<()>,
	/// Counts this transaction on `FS_TRANSACTIONS` while it is pending.
	pub _count_metric: IntGaugeGuard,
	/// Counts `bytes` on `FS_TRANSACTION_BYTES`; replaced whenever `bytes` grows.
	pub bytes_metric: Option<IntGaugeGuard>,
}

impl Pending {
	/// Adds `bytes` of pending layers to this transaction and its byte gauge.
	pub(crate) fn add_bytes(&mut self, bytes: usize) {
		self.bytes += bytes;

		// Release the old count before taking the new one.
		self.bytes_metric = None;
		self.bytes_metric = Some(
			crate::metrics::FS_TRANSACTION_BYTES
				.add_guard(self.bytes.min(i64::MAX as usize) as i64),
		);
	}
}

/// Not an optimistic KV transaction: this handle exclusively owns its declared
/// inode footprint, and cannot survive a new claim of the volume.
pub struct TxnHandle {
	fs: Arc<Filesystem>,
	id: Vec<u8>,
	owner: TransactionOwner,
	finished: bool,
	/// Set when one of this transaction's appends ended with an unknown outcome. The entry may
	/// or may not be in the log, so the transaction can no longer say what it would commit:
	/// every later append and the commit are refused, and the caller aborts and retries it.
	uncertain: std::sync::atomic::AtomicBool,
}
impl Filesystem {
	pub async fn begin_txn(
		self: &Arc<Self>,
		footprint: impl IntoIterator<Item = u64>,
	) -> Result<TxnHandle> {
		self.begin_transaction(footprint.into_iter().collect(), None)
			.await
	}
	/// Begins a staged write of `ino` under a stable identity derived from the inode and
	/// `base`, the inode's change sequence the write builds on, so an owner that lost its handle
	/// can find and abort the same transaction with [`Self::abort_staged_write`]. The
	/// transaction holds `ino` and every inode in `extra_inodes`, so its entries may also write
	/// those inodes, and any other writer of them waits until it commits or aborts.
	pub async fn begin_staged_write(
		self: &Arc<Self>,
		ino: u64,
		base: u64,
		extra_inodes: impl IntoIterator<Item = u64>,
	) -> Result<TxnHandle> {
		let mut footprint = BTreeSet::from([ino]);
		footprint.extend(extra_inodes);
		self.begin_transaction(footprint, Some(crate::keys::staged_write_txn(ino, base)))
			.await
	}
	async fn begin_transaction(
		self: &Arc<Self>,
		footprint: BTreeSet<u64>,
		id: Option<Vec<u8>>,
	) -> Result<TxnHandle> {
		// Transaction entries cannot carry a staged format, so write it first.
		self.write_staged_format().await?;
		let deadline = self.deadline()?;
		loop {
			let wake = self.transaction_wake.notified();
			tokio::pin!(wake);
			wake.as_mut().enable();
			let write =
				rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
			let mut state = self.state.lock().await;
			let status = self.refresh(&mut state, deadline).await?;
			if state
				.transactions
				.values()
				.any(|t| !t.footprint.is_disjoint(&footprint))
			{
				drop(state);
				drop(write);
				rivet_env::timeout_at(self.clock.as_ref(), deadline, wake)
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
				continue;
			}
			self.shed_unapplied_backlog(&mut state, deadline).await?;
			if state.transactions.len() >= self.config.max_pending_commits {
				crate::metrics::FS_OVERLOADED
					.with_label_values(&["pending_transactions"])
					.inc();
				return Err(Error::Overloaded);
			}
			state.next_transaction = state
				.next_transaction
				.checked_add(1)
				.ok_or_else(|| corrupt!())?;
			let id = id.unwrap_or_else(|| {
				crate::keys::client_txn(status.client_epoch, state.next_transaction)
			});
			if state.transactions.contains_key(&id) {
				return Err(Error::Busy);
			}
			let owner = TransactionOwner {
				reservation: state.next_transaction,
			};
			let expires = self
				.now()?
				.boot_ms
				.saturating_add(self.config.max_transaction_age_ms);
			let reserve = self.budget.reserve(4096)?;
			let (expiry, cancelled) = tokio::sync::oneshot::channel();
			state.transactions.insert(
				id.clone(),
				Pending {
					owner,
					footprint,
					layers: Vec::new(),
					bytes: 0,
					expires,
					abort_reserve: Some(reserve),
					_expiry: expiry,
					_count_metric: crate::metrics::FS_TRANSACTIONS.inc_guard(),
					bytes_metric: None,
				},
			);
			let weak = Arc::downgrade(self);
			let timeout_id = id.clone();
			let clock = self.clock.clone();
			if let Err(error) = self.journal.spawn_cleanup(Box::pin(async move {
				if rivet_env::timeout_at(clock.as_ref(), expires, cancelled)
					.await
					.is_err()
				{
					if let Some(fs) = weak.upgrade() {
						tracing::warn!("filesystem transaction exceeded its maximum age");
						crate::metrics::FS_TRANSACTION_ABORTS
							.with_label_values(&["age"])
							.inc();
						fs.abort_until_settled(&timeout_id, owner).await;
					}
				}
			})) {
				state.transactions.remove(&id);
				return Err(error);
			}
			return Ok(TxnHandle {
				fs: self.clone(),
				id,
				owner,
				finished: false,
				uncertain: std::sync::atomic::AtomicBool::new(false),
			});
		}
	}
	pub(super) fn footprint_busy(state: &State, entry: &FsEntry, seq: u64) -> bool {
		let footprint = txn::footprint(seq, &entry.ops);
		let own = entry.txn.as_ref().map(txn::id);
		state.transactions.iter().any(|(id, pending)| {
			Some(id.as_slice()) != own && !pending.footprint.is_disjoint(&footprint)
		})
	}
	fn check_transaction<'a>(
		&self,
		state: &'a State,
		id: &[u8],
		owner: TransactionOwner,
	) -> Result<&'a Pending> {
		let pending = state
			.transactions
			.get(id)
			.filter(|t| t.owner == owner)
			.ok_or_else(|| self.fenced(crate::fences::FenceReason::TransactionMissing))?;
		self.status()?;
		if self.now()?.boot_ms >= pending.expires {
			return Err(Error::DeadlineExceeded);
		}
		Ok(pending)
	}
	fn transaction_view(&self, state: &State, id: &[u8], owner: TransactionOwner) -> Result<View> {
		let pending = self.check_transaction(state, id, owner)?;
		let mut view = self.view(state, state.last_seq);
		view.layers.extend(pending.layers.iter().cloned());
		Ok(view)
	}
	/// Aborts a transaction from background cleanup. An abort that ends undecided, such as one
	/// waiting out a stalled quorum past its deadline, is latency rather than a lost claim, so
	/// it is tried again after a delay while the same history serves, each try with its own
	/// deadline. Any other failure fences, because the transaction could otherwise stay open.
	async fn abort_until_settled(&self, id: &[u8], owner: TransactionOwner) {
		loop {
			let Err(error) = self.abort_transaction(id, owner).await else {
				return;
			};
			let undecided = Self::commit_undecided(&error)
				&& !self.fenced.load(std::sync::atomic::Ordering::Acquire)
				&& self.status().is_ok();
			if !undecided {
				self.fence_with(crate::fences::FenceReason::TransactionAbortFailed);
				return;
			}
			tracing::debug!(?error, "transaction abort is undecided; trying again");
			let Ok(now) = self.now() else {
				self.fence_with(crate::fences::FenceReason::TransactionAbortFailed);
				return;
			};
			let retry_at = now.boot_ms.saturating_add(ABORT_RETRY_MS);
			if self.clock.sleep_until(retry_at).await.is_err() {
				self.fence_with(crate::fences::FenceReason::TransactionAbortFailed);
				return;
			}
		}
	}

	async fn abort_transaction(&self, id: &[u8], owner: TransactionOwner) -> Result<()> {
		let deadline = self.deadline()?;
		let _write = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut state = self.state.lock().await;
		// An entry of this transaction whose outcome is unknown, such as its commit, is
		// installed first, so an abort never follows a commit the log already holds.
		if let Ok(status) = self.status() {
			self.reconcile_unapplied(&mut state, status, deadline)
				.await?;
		}
		if state
			.transactions
			.get(id)
			.is_none_or(|pending| pending.owner != owner)
		{
			return Ok(());
		}
		let status = self.status()?;
		crate::metrics::FS_TRANSACTION_ABORTS
			.with_label_values(&["abort"])
			.inc();
		// Reserved at begin, so ordinary budget pressure cannot prevent abort.
		state
			.transactions
			.get_mut(id)
			.ok_or(Error::Fenced)?
			.abort_reserve
			.take();
		let receipt = self
			.admit(
				&mut state,
				FsEntry {
					ops: Vec::new(),
					txn: Some(FsTxn::FsTxnAbort(FsTxnAbort {
						id: id.to_vec().into(),
					})),
				},
				status,
				deadline,
			)
			.await?;
		drop(state);
		self.finish_transaction(receipt, deadline).await.map(|_| ())
	}
	async fn finish_transaction(
		&self,
		receipt: super::recovery::Receipt<'_>,
		deadline: u64,
	) -> Result<u64> {
		let super::recovery::Receipt {
			outcome,
			ticket,
			attempt,
			..
		} = receipt;
		let seq = outcome.volume_seq;
		let result = async {
			rivet_env::timeout_at(
				self.clock.as_ref(),
				deadline,
				ticket.ok_or_else(|| corrupt!())??,
			)
			.await
			.map_err(|_| Error::DeadlineExceeded)?
			.map_err(Error::Log)
		}
		.await;
		// The ticket proves this commit. A wait still undecided at the deadline keeps serving,
		// and the entry commits later in its place.
		let result = self.settle_commit(attempt, seq, result, false);
		result.map_err(|error| Self::admitted_error(error, outcome.token))?;
		Ok(seq)
	}
	/// Ends the transactions an older writer left open with one abort entry at this writer's
	/// epoch. Used while opening, before any caller can begin a transaction.
	pub(super) async fn abort_abandoned(&self, id: Vec<u8>, deadline: u64) -> Result<()> {
		let mut state = self.state.lock().await;
		let status = self.status()?;
		let receipt = self
			.admit(
				&mut state,
				FsEntry {
					ops: vec![],
					txn: Some(FsTxn::FsTxnAbort(FsTxnAbort { id: id.into() })),
				},
				status,
				deadline,
			)
			.await?;
		drop(state);
		self.finish_transaction(receipt, deadline).await.map(|_| ())
	}
	/// Aborts the staged write of `ino` on `base` whose handle its owner no longer holds, as
	/// when the owner restarts a write it cannot finish.
	pub async fn abort_staged_write(&self, ino: u64, base: u64) -> Result<()> {
		let id = crate::keys::staged_write_txn(ino, base);
		let deadline = self.deadline()?;
		let _write = self.write_gate.lock().await;
		let mut state = self.state.lock().await;
		let status = self.refresh(&mut state, deadline).await?;
		if let Some(pending) = state.transactions.get_mut(&id) {
			pending.abort_reserve.take();
		}
		let receipt = self
			.admit(
				&mut state,
				FsEntry {
					ops: vec![],
					txn: Some(FsTxn::FsTxnAbort(FsTxnAbort { id: id.into() })),
				},
				status,
				deadline,
			)
			.await?;
		drop(state);
		self.finish_transaction(receipt, deadline).await.map(|_| ())
	}
	pub fn max_transaction_bytes(&self) -> usize {
		self.config.max_transaction_bytes
	}
}
impl TxnHandle {
	async fn check(&self, deadline: u64) -> Result<()> {
		let state = rivet_env::timeout_at(self.fs.clock.as_ref(), deadline, self.fs.state.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		self.fs
			.check_transaction(&state, &self.id, self.owner)
			.map(|_| ())
	}
	pub async fn getattr(&self, ino: u64) -> Result<Inode> {
		let deadline = self.fs.deadline()?;
		let state = rivet_env::timeout_at(self.fs.clock.as_ref(), deadline, self.fs.state.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut view = self.fs.transaction_view(&state, &self.id, self.owner)?;
		drop(state);
		let inode = self
			.fs
			.evaluate(&mut view, deadline, |v| v.inode(ino))
			.await?;
		self.check(deadline).await?;
		Ok(inode)
	}
	pub async fn read(&self, ino: u64, offset: u64, count: usize) -> Result<Vec<u8>> {
		if count > self.fs.config.max_operation_bytes {
			return Err(Error::TooLarge);
		}
		let deadline = self.fs.deadline()?;
		let state = rivet_env::timeout_at(self.fs.clock.as_ref(), deadline, self.fs.state.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut view = self.fs.transaction_view(&state, &self.id, self.owner)?;
		drop(state);
		let bytes = self
			.fs
			.evaluate(&mut view, deadline, |v| {
				rules::read(v, ino, offset, count, None)
			})
			.await?;
		self.check(deadline).await?;
		Ok(bytes)
	}
	pub async fn edit_file(&mut self, ino: u64, edits: &[FileEdit]) -> Result<u64> {
		let _scratch = self.fs.reserve_edits(edits, &[], &[])?;
		self.append(false, |v, seq, now| {
			file_batch::build(v, seq, now, ino, edits, &[], &[])
		})
		.await
	}
	pub async fn mutate_in(&mut self, ops: Vec<FsOp>) -> Result<u64> {
		self.append(false, |_, _, _| Ok(ops.clone())).await
	}
	pub async fn commit(mut self, ops: Vec<FsOp>) -> Result<u64> {
		let result = self.append(true, |_, _, _| Ok(ops.clone())).await;
		self.finished = result.is_ok();
		result
	}
	pub async fn commit_file(mut self, ino: u64, size: u64) -> Result<u64> {
		self.commit_file_with_xattrs(ino, size, &[]).await
	}

	/// Commits like [`Self::commit_file`], and sets `xattrs` on the file in the same final
	/// entry, so they become visible with the transaction's writes. The handle stays with its
	/// caller: after a commit refused before it was admitted, such as one whose cold read was
	/// refused, the transaction is still open and the identical commit may be sent again.
	/// Dropping the handle after any other failure aborts the transaction, as before.
	pub async fn commit_file_with_xattrs(
		&mut self,
		ino: u64,
		size: u64,
		xattrs: &[(Vec<u8>, Vec<u8>)],
	) -> Result<u64> {
		self.commit_file_with_ops(ino, size, xattrs, Vec::new())
			.await
	}

	/// Commits like [`Self::commit_file_with_xattrs`], with `extra` operations in the same
	/// final entry, after the file's size and attributes, so rows on the transaction's other
	/// inodes become visible with its writes. Every inode the extra operations touch must be
	/// in the footprint the transaction began with, or the commit is refused as invalid before
	/// anything is appended. The handle stays with its caller as for
	/// [`Self::commit_file_with_xattrs`]: a commit refused before admission leaves the
	/// transaction open, and the identical commit may be sent again.
	pub async fn commit_file_with_ops(
		&mut self,
		ino: u64,
		size: u64,
		xattrs: &[(Vec<u8>, Vec<u8>)],
		extra: Vec<FsOp>,
	) -> Result<u64> {
		self.commit_file_edits_with_ops(ino, &[FileEdit::Truncate(size)], xattrs, extra)
			.await
	}

	/// Publishes ordered final file edits and extra operations with all staged writes.
	pub async fn commit_file_edits_with_ops(
		&mut self,
		ino: u64,
		edits: &[FileEdit],
		xattrs: &[(Vec<u8>, Vec<u8>)],
		extra: Vec<FsOp>,
	) -> Result<u64> {
		let _scratch = self.fs.reserve_edits(edits, xattrs, &extra)?;
		let result = self
			.append(true, |v, seq, now| {
				file_batch::build(v, seq, now, ino, &edits, xattrs, &extra)
			})
			.await;
		self.finished = result.is_ok();
		result
	}
	pub async fn abort(mut self) -> Result<()> {
		let result = self.fs.abort_transaction(&self.id, self.owner).await;
		self.finished = result.is_ok();
		result
	}
	/// Appends one entry of this transaction, refusing once an earlier append's outcome is
	/// unknown; see [`Self::uncertain`].
	async fn append(
		&self,
		commit: bool,
		build: impl Fn(&View, u64, i64) -> depot_fs_core::Result<Vec<FsOp>>,
	) -> Result<u64> {
		use std::sync::atomic::Ordering;
		if self.uncertain.load(Ordering::Acquire) {
			return Err(Error::Conflict);
		}
		let result = self.append_entry(commit, build).await;
		if let Err(error) = &result
			&& Filesystem::outcome_unknown(error)
		{
			self.uncertain.store(true, Ordering::Release);
		}
		result
	}

	async fn append_entry(
		&self,
		commit: bool,
		build: impl Fn(&View, u64, i64) -> depot_fs_core::Result<Vec<FsOp>>,
	) -> Result<u64> {
		let fs = &self.fs;
		let deadline = fs.deadline()?;
		let _write = rivet_env::timeout_at(fs.clock.as_ref(), deadline, fs.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		let mut state = fs.state.lock().await;
		// Do not call refresh/current here: a new claim invalidates this transaction,
		// including when recovery reuses its epoch. Never restart its pending writes. An
		// admission whose outcome is unknown is installed first, so this entry is built on
		// the transaction view the log holds and admitted at the sequence it was built for.
		let status = fs.status()?;
		let status = fs.reconcile_unapplied(&mut state, status, deadline).await?;
		let mut view = fs.transaction_view(&state, &self.id, self.owner)?;
		let ops = fs
			.evaluate(&mut view, deadline, |v| {
				build(
					v,
					status.next_seq,
					fs.now().map_err(|_| depot_fs_core::Error::Corrupt)?.unix_ms,
				)
			})
			.await?;
		drop(view);
		fs.check_transaction(&state, &self.id, self.owner)?;
		let entry = FsEntry {
			ops,
			txn: Some(if commit {
				FsTxn::FsTxnCommit(FsTxnCommit {
					id: self.id.clone().into(),
				})
			} else {
				FsTxn::FsTxnPending(FsTxnPending {
					id: self.id.clone().into(),
				})
			}),
		};
		let pending = state.transactions.get(&self.id).ok_or_else(|| {
			self.fs
				.fenced(crate::fences::FenceReason::TransactionMissing)
		})?;
		let mut footprint = txn::footprint(status.next_seq, &entry.ops);
		if commit && entry.ops.iter().any(txn::creates) {
			footprint.remove(&status.next_seq);
		}
		if !footprint.is_subset(&pending.footprint) {
			return Err(depot_fs_core::Error::Invalid.into());
		}
		if commit && pending.layers.is_empty() {
			// No pending entry means a commit is a no-op. Open an empty pending
			// entry so a transaction with only final operations has real meaning.
			let receipt = fs
				.admit(
					&mut state,
					FsEntry {
						ops: vec![],
						txn: Some(FsTxn::FsTxnPending(FsTxnPending {
							id: self.id.clone().into(),
						})),
					},
					status,
					deadline,
				)
				.await?;
			fs.check_transaction(&state, &self.id, self.owner)?;
			receipt.attempt.finish();
		}
		fs.check_transaction(&state, &self.id, self.owner)?;
		let receipt = fs.admit(&mut state, entry, fs.status()?, deadline).await?;
		let seq = receipt.outcome.volume_seq;
		if commit {
			drop(state);
			fs.finish_transaction(receipt, deadline).await
		} else {
			fs.check_transaction(&state, &self.id, self.owner)?;
			receipt.attempt.finish();
			Ok(seq)
		}
	}
}
impl Drop for TxnHandle {
	fn drop(&mut self) {
		if self.finished {
			return;
		}
		let fs = self.fs.clone();
		let id = self.id.clone();
		let owner = self.owner;
		let cleanup = Box::pin(async move {
			fs.abort_until_settled(&id, owner).await;
		});
		if self.fs.journal.spawn_cleanup(cleanup).is_err() {
			self.fs
				.fence_with(crate::fences::FenceReason::TransactionAbortFailed);
		}
	}
}
