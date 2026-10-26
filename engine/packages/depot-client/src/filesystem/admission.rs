//! Validate before log admission, then install one invisible or visible overlay.
use super::{
	Error, Filesystem, Result,
	cache::Layer,
	client::{Attempt, State},
	transport::LogStatus,
};
use depot_fs_core::{store::Row, *};
use depot_volume::volume_client_machine::types::ErrorDisposition;
use rivet_data::versioned::DepotFsEntryData;
use vbare::Versioned;
impl Filesystem {
	pub(super) async fn admit<'a>(
		&'a self,
		state: &mut State,
		entry: FsEntry,
		status: LogStatus,
		deadline: u64,
	) -> Result<super::recovery::Receipt<'a>> {
		// Entries of this opening's own epoch whose admission outcome it never observed are
		// read back and installed first, so this entry extends the view the log holds.
		let status = self.reconcile_unapplied(state, status, deadline).await?;
		if status.next_seq != state.last_seq.checked_add(1).ok_or_else(|| corrupt!())? {
			self.fence_with(crate::fences::FenceReason::AdmitSequenceMismatch);
			return Err(Error::Fenced);
		}
		// Staged entries before the last are admitted first, each on its own. They are not
		// awaited: this entry follows them in the log, so its commit covers them. A created
		// inode is numbered by its entry's sequence. Admit the last staged entry separately
		// when this entry creates another inode or changes the staged inode's postimage:
		// creation needs a distinct number, and a first write must advance the published
		// inode version. KV row writes can still share the inode's creation entry.
		let changes_staged_inode = if let Some(last) = state
			.staged_format
			.as_ref()
			.and_then(|staged| staged.entries.last())
		{
			let staged = apply::plan(
				last.seq,
				&FsEntry {
					txn: None,
					ops: last.ops.clone(),
				},
			)?;
			let update = apply::plan(status.next_seq, &entry)?;
			staged.final_rows.rows.keys().any(|key| {
				matches!(key, store::RowKey::Inode(_)) && update.final_rows.rows.contains_key(key)
			})
		} else {
			false
		};
		let mut status = status;
		while state.staged_format.as_ref().is_some_and(|staged| {
			staged.entries.len() > 1
				|| staged.entries.first().is_some_and(|last| {
					(creates_inode(&entry.ops)
						&& (creates_inode(&last.ops) || last.seq == ROOT_INO))
						|| changes_staged_inode
				})
		}) {
			let Some(staged) = state.staged_format.as_mut() else {
				break;
			};
			let head = staged.entries.remove(0);
			if head.seq != status.next_seq {
				self.fence_with(crate::fences::FenceReason::StagedFormatMismatch);
				return Err(Error::Fenced);
			}
			let rest = state.staged_format.take();
			let admitted = Box::pin(self.admit(
				state,
				FsEntry {
					txn: None,
					ops: head.ops.clone(),
				},
				status,
				deadline,
			))
			.await;
			match admitted {
				Ok(receipt) => {
					receipt.attempt.finish();
					state.staged_format = rest.filter(|staged| !staged.entries.is_empty());
				}
				Err(error) => {
					// A staged entry whose outcome is unknown cannot be installed in place,
					// because the format must be written exactly once, so it fences this
					// opening. A refusal admitted nothing. Either way the staging stays whole
					// for an exact retry.
					if Self::outcome_unknown(&error) {
						self.fence_with(crate::fences::FenceReason::StagedFormatMismatch);
					}
					let mut restored = rest.ok_or_else(|| corrupt!())?;
					restored.entries.insert(0, head);
					state.staged_format = Some(restored);
					return Err(error);
				}
			}
			status = self.journal.status()?;
			if status.next_seq != state.last_seq.checked_add(1).ok_or_else(|| corrupt!())? {
				self.fence_with(crate::fences::FenceReason::AdmitSequenceMismatch);
				return Err(Error::Fenced);
			}
		}
		// The last staged entry merges with this one. Its inode numbers are its sequence, so
		// any other position fences this opening.
		let entry = match &state.staged_format {
			None => entry,
			Some(staged) => match staged.entries.as_slice() {
				[last] if last.seq == status.next_seq && entry.txn.is_none() => {
					let mut ops = last.ops.clone();
					ops.extend(entry.ops);
					FsEntry { txn: None, ops }
				}
				_ => {
					self.fence_with(crate::fences::FenceReason::StagedFormatMismatch);
					return Err(Error::Fenced);
				}
			},
		};
		// The entry is encoded once. The journal, the receipt and the volume client share these
		// bytes, and pages inside them are never copied again.
		let payload = DepotFsEntryData::seal_to_bytes(&entry, self.journal.entry_version()?)
			.map_err(|_| corrupt!())?;
		if payload.len() > status.max_payload_bytes
			|| payload.len() > self.config.max_operation_bytes
		{
			return Err(Error::TooLarge);
		}
		let _payload_charge = self.budget.reserve(payload.len())?;
		let retained = self.budget.reserve(347)?;
		txn::validate(&entry)?;
		if Self::footprint_busy(state, &entry, status.next_seq) {
			return Err(Error::Busy);
		}
		let batch = apply::plan(status.next_seq, &entry)?;
		let bytes = row_bytes(&batch);
		let pending_entry = matches!(entry.txn, Some(FsTxn::FsTxnPending(_)));
		let id = entry.txn.as_ref().map(txn::id);
		if let Some(pending) = id.and_then(|id| state.transactions.get(id)) {
			if pending.bytes.saturating_add(bytes) > self.config.max_transaction_bytes {
				tracing::warn!(
					limit = self.config.max_transaction_bytes,
					"filesystem transaction exceeds its byte cap"
				);
				return Err(Error::TooLarge);
			}
		}
		// Resolve the entry already in hand instead of decoding the payload just encoded.
		let mut resolver = state.resolver.clone();
		let resolution = resolver.push_entry(
			status.next_seq,
			status.client_epoch,
			entry.clone(),
			payload.len() as u64,
		)?;
		// Reserve before building so the indexed fallback takes the plan without a copy.
		let layer = if entry.txn.is_none() {
			match Layer::charge_cached(&batch, &self.budget, self.config.max_cache_bytes) {
				Some(charge) => Layer::with_charge(status.next_seq, batch, charge),
				None => Layer::indexed(status.next_seq, batch, &self.budget, self.journal.clone())?,
			}
		} else {
			Layer::indexed(status.next_seq, batch, &self.budget, self.journal.clone())?
		};
		let promoted = if let txn::Resolution::Commit { id, .. } = &resolution {
			let pending = state
				.transactions
				.get(id)
				.ok_or_else(|| self.fenced(crate::fences::FenceReason::TransactionMissing))?;
			let mut layers = pending.layers.clone();
			layers.push(layer.clone());
			Some(Layer::merge(status.next_seq, &layers, &self.budget)?)
		} else {
			None
		};
		let change = match &resolution {
			txn::Resolution::Standalone(_) => Some(super::transaction::Change::new(
				status.next_seq,
				self.now()?.boot_ms,
				&entry,
				&self.budget,
			)?),
			txn::Resolution::Commit { open, .. } => {
				let mut footprint = open.footprint.clone();
				footprint.extend(txn::footprint(status.next_seq, &entry.ops));
				Some(super::transaction::Change::footprint(
					status.next_seq,
					self.now()?.boot_ms,
					&footprint,
					&self.budget,
				)?)
			}
			_ => None,
		};
		// Bound the uncommitted window independently of cold progress. The
		// barrier frees replica payloads even while this transaction pins cold.
		if status.retained_bytes.saturating_add(payload.len()) > self.config.max_memory_bytes / 4 {
			rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.barrier()?)
				.await
				.map_err(|_| Error::DeadlineExceeded)?
				.map_err(Error::Log)?;
		}

		let attempt = Attempt::new(self);
		let result = rivet_env::timeout_at(
			self.clock.as_ref(),
			deadline,
			self.journal.admit(payload.clone()),
		)
		.await
		.unwrap_or(Err(Error::DeadlineExceeded));
		let outcome = match result {
			Ok(value) => value,
			Err(Error::Overloaded) => {
				attempt.finish();
				return Err(Error::Overloaded);
			}
			Err(Error::Log(
				error @ depot_volume::volume_client_machine::types::WriteError::HistoryBlocked {
					..
				},
			)) => {
				// Nothing was dispatched, but nothing can commit on this volume client any
				// more: fence this filesystem so its owner reopens past the blocked entry.
				attempt.finish();
				self.fence_with(crate::fences::FenceReason::HistoryBlocked);
				return Err(Error::Log(error));
			}
			Err(Error::Log(error)) if error.disposition() == ErrorDisposition::NotExecuted => {
				// Nothing was dispatched, so the refusal leaves no uncertain write and keeps
				// this opening usable. The edge owns retries of a refused request.
				attempt.finish();
				return Err(Error::Log(error));
			}
			Err(error)
				if Self::outcome_unknown(&error)
					&& state.staged_format.is_none()
					&& self.status().is_ok() =>
			{
				// Admission may still complete. Retain its exact payload for readback on
				// the next request, instead of resubmitting an uncertain write.
				crate::metrics::FS_UNKNOWN_OUTCOMES
					.with_label_values(&["admit"])
					.inc();
				state.unobserved_admission = Some(payload);
				attempt.finish();
				return Err(error);
			}
			Err(error) => return Err(error),
		};
		// The journal never changes volume client under this filesystem, so its history is
		// the one this write was admitted into unless the journal stopped serving.
		let journal_serving = self.status().is_ok();
		// Multipart entries cannot be resumed on a journal that stopped serving, even at the
		// same sequence. Standalone writes retain their receipt for token-based recovery.
		if !journal_serving && entry.txn.is_some() {
			return Err(self.fenced(crate::fences::FenceReason::AdmitReopened));
		}
		if outcome.volume_seq != status.next_seq || outcome.applied_earlier {
			// The sequence this filesystem expected was taken by an entry it did not admit,
			// so this view is stale: the armed attempt fences the filesystem and its owner
			// opens a new one that replays the log. A journal that cannot report its status
			// is fenced, which is no evidence of corruption either.
			if !journal_serving {
				return Err(self.fenced(crate::fences::FenceReason::AdmitReopened));
			}
			return Err(corrupt!());
		}
		state.last_seq = outcome.volume_seq;
		state
			.backlog
			.admitted(outcome.volume_seq, payload.len() as u64);
		// The admitted entry carries the format. If it never becomes durable, the attempt
		// fences this opening and the next one stages again or finds the root.
		state.staged_format = None;
		state
			.positions
			.insert(outcome.volume_seq, (status.client_epoch, retained));
		state.resolver = resolver;
		match resolution {
			txn::Resolution::Standalone(_) => self.push_layer(state, layer),
			txn::Resolution::Pending => {
				let pending = state
					.transactions
					.get_mut(id.ok_or_else(|| corrupt!())?)
					.ok_or_else(|| self.fenced(crate::fences::FenceReason::TransactionMissing))?;
				pending.layers.push(layer);
				pending.add_bytes(bytes);
			}
			txn::Resolution::Commit { id, .. } => {
				self.push_layer(state, promoted.ok_or_else(|| corrupt!())?);
				state.transactions.remove(&id);
				self.transaction_wake.notify_waiters();
			}
			txn::Resolution::Discarded => {
				if let Some(id) = id {
					state.transactions.remove(id);
				}
				self.transaction_wake.notify_waiters();
			}
		}
		if let Some(change) = change {
			state.conflicts.push(change, &self.config);
		}
		// Pending entries need only admission; the closing ticket covers their durability.
		let ticket = (!pending_entry).then(|| self.journal.barrier());
		Ok(super::recovery::Receipt {
			outcome,
			payload,
			ticket,
			attempt,
		})
	}

	/// Whether an admission or commit wait failed without deciding the entry: the deadline
	/// passed, or the volume client answered with an unknown outcome. A fence is evidence of
	/// a newer claim and a corrupt answer is a broken client, so neither counts.
	pub(super) fn outcome_unknown(error: &Error) -> bool {
		use depot_volume::volume_client_machine::types::{ErrorDisposition, WriteError};
		match error {
			Error::DeadlineExceeded => true,
			Error::Log(WriteError::Fenced { .. }) => false,
			Error::Log(error) => error.disposition() == ErrorDisposition::OutcomeUnknown,
			_ => false,
		}
	}
}

/// Application bytes a planned entry writes, counted against a transaction's byte cap.
pub(super) fn row_bytes(batch: &apply::ApplyPlan) -> usize {
	batch
		.final_rows
		.rows
		.values()
		.map(|row| match row {
			Some(Row::Bytes(bytes)) => bytes.len(),
			Some(Row::Inode(inode)) => inode.inline.as_ref().map_or(0, |inline| inline.len()),
			_ => 0,
		})
		.sum::<usize>()
}

/// Whether `ops` create an inode, which takes its entry's sequence as its number.
fn creates_inode(ops: &[FsOp]) -> bool {
	ops.iter().any(|op| {
		matches!(
			op,
			FsOp::FsCreate(_) | FsOp::FsMkdir(_) | FsOp::FsMknod(_) | FsOp::FsSymlink(_)
		)
	})
}
