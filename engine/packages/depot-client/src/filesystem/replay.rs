//! Bounded replay uses the same visibility resolver as the cold-store maintainer. The same
//! read-back installs this opening's own entries whose admission outcome it never observed.
use super::{
	Error, Filesystem, Result, cache::Layer, client::State, transaction::Change,
	transport::LogStatus,
};
use depot_fs_core::{
	FsTxn, apply,
	txn::{Resolution, Resolver},
};
use depot_volume::volume_client_machine::runner::LogHistory;
use rivet_data::{
	generated::{
		depot_log_shard_v1::{PayloadCodec, PayloadWrite, VolumeLogEntry, VolumeLogEntryBody},
		shared_depot_types_v1::VolumeLogPosition,
	},
	versioned::DepotFsEntryData,
};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use vbare::Versioned;

impl Filesystem {
	pub(super) async fn replay(
		&self,
		state: &mut State,
		history: LogHistory,
		start: VolumeLogPosition,
		deadline: u64,
	) -> Result<()> {
		state.resolver = Resolver::new(start.clone());
		state.last_seq = start.volume_seq;
		let mut pending = BTreeMap::<Vec<u8>, Vec<_>>::new();
		let mut entries = history.entries;
		entries.retain(|entry| entry.volume_seq > start.volume_seq);
		while state.last_seq < history.committed {
			if entries.is_empty() {
				let page = rivet_env::timeout_at(
					self.clock.as_ref(),
					deadline,
					self.journal
						.read(state.last_seq + 1, self.status()?.max_payload_bytes + 128),
				)
				.await
				.map_err(|_| Error::DeadlineExceeded)??;
				if page.cold.volume_seq > state.last_seq {
					return Err(Error::TooOld);
				}
				entries = page.entries;
				if entries.is_empty() {
					return Err(corrupt!());
				}
			}
			for log in entries
				.drain(..)
				.take_while(|e| e.volume_seq <= history.committed)
			{
				let resolution = state.resolver.push(&log)?;
				match resolution {
					Resolution::Standalone(entry) => self.push_layer(
						state,
						Layer::indexed(
							log.volume_seq,
							apply::plan(log.volume_seq, &entry)?,
							&self.budget,
							self.journal.clone(),
						)?,
					),
					Resolution::Pending => {
						let entry = decode(&log)?;
						let Some(FsTxn::FsTxnPending(txn)) = &entry.txn else {
							return Err(corrupt!());
						};
						pending
							.entry(txn.id.to_vec())
							.or_default()
							.push(Layer::indexed(
								log.volume_seq,
								apply::plan(log.volume_seq, &entry)?,
								&self.budget,
								self.journal.clone(),
							)?);
					}
					Resolution::Commit { id, entry, .. } => {
						let mut layers = pending.remove(&id).ok_or_else(|| corrupt!())?;
						layers.push(Layer::indexed(
							log.volume_seq,
							apply::plan(log.volume_seq, &entry)?,
							&self.budget,
							self.journal.clone(),
						)?);
						self.push_layer(
							state,
							Layer::merge(log.volume_seq, &layers, &self.budget)?,
						);
					}
					Resolution::Discarded => {
						pending.retain(|id, _| state.resolver.open().contains_key(id))
					}
				}
				state.last_seq = log.volume_seq;
				state.positions.insert(
					log.volume_seq,
					(log.client_epoch, self.budget.reserve(256)?),
				);
			}
		}
		// Transactions an older writer left open end at this writer's first entry, which the
		// open writes when any remain. Never leave one open in this writer's own epoch.
		if !state.resolver.only_older_open(self.status()?.client_epoch) {
			self.fence_with(crate::fences::FenceReason::ReplayOpenTransaction);
			return Err(Error::Fenced);
		}
		Ok(())
	}

	/// Resolves this opening's admission whose outcome it did not observe, such as one that
	/// passed its deadline, by reading the log back at the same epoch.
	///
	/// Admission is serialized under the state lock and every later admission resolves this one
	/// first, so at most one is unobserved, and if it lands it is the next entry above
	/// `last_seq`, validated against the view it extends. Until a barrier orders behind it the
	/// volume client may still admit it, so the barrier comes first: it travels the same inbox
	/// and waits for every admitted entry to commit. Then the log holds either nothing new, so
	/// the write never landed, or exactly that entry, which is read back and installed as its
	/// admission would have, so the view matches what a reopen would replay.
	///
	/// An unknown outcome is latency, not evidence of a newer claim, so this never fences on
	/// it. The barrier and the reads are bounded by `deadline`; a deadline that passes keeps the
	/// admission unobserved for the next request. Anything else above `last_seq`, a different
	/// epoch or claim entry, or a staged format is not this opening's own admission or cannot be
	/// installed in place, and fences. When cold already passed a landed standalone entry, it
	/// is installed from the payload this opening kept; a transaction entry there fences.
	///
	/// Returns the status to admit against, which is `status` when nothing was unobserved.
	pub(super) async fn reconcile_unapplied(
		&self,
		state: &mut State,
		status: LogStatus,
		deadline: u64,
	) -> Result<LogStatus> {
		self.reconcile_unapplied_with(state, status, deadline, true)
			.await
	}

	/// [`Self::reconcile_unapplied`]; `settle` makes it first wait for undecided commits,
	/// which a request whose view stops at the committed position does not need.
	pub(super) async fn reconcile_unapplied_with(
		&self,
		state: &mut State,
		status: LogStatus,
		deadline: u64,
		settle: bool,
	) -> Result<LogStatus> {
		let status = match settle {
			true => self.settle_undecided(status, deadline).await?,
			false => status,
		};
		let expected = state.last_seq.checked_add(1).ok_or_else(|| corrupt!())?;
		if status.next_seq <= expected && state.unobserved_admission.is_none() {
			return Ok(status);
		}
		if state.unobserved_admission.is_none() || state.staged_format.is_some() {
			self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
			return Err(Error::Fenced);
		}

		// Order behind the unobserved admission and wait for it to commit if it landed.
		let ticket = self.journal.barrier()?;
		rivet_env::timeout_at(self.clock.as_ref(), deadline, ticket)
			.await
			.map_err(|_| Error::DeadlineExceeded)?
			.map_err(Error::Log)?;
		let status = self.status()?;
		if status.next_seq == expected {
			state.unobserved_admission = None;
			tracing::debug!(
				last_seq = state.last_seq,
				"an admission with an unknown outcome never landed"
			);
			return Ok(status);
		}
		if status.next_seq != expected + 1 {
			self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
			return Err(Error::Fenced);
		}

		// Read the landed entry back and check it is this opening's own write.
		let page = rivet_env::timeout_at(
			self.clock.as_ref(),
			deadline,
			self.journal.read(expected, status.max_payload_bytes + 128),
		)
		.await
		.map_err(|_| Error::DeadlineExceeded)??;
		let found = page
			.entries
			.into_iter()
			.find(|entry| entry.volume_seq == expected);
		let installed = match found {
			Some(log) => {
				let raw = raw_payload(&log)?;
				let unobserved = state.unobserved_admission.as_deref();
				if log.client_epoch != state.client_epoch || unobserved != Some(raw.as_slice()) {
					self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
					return Err(Error::Fenced);
				}
				self.install_read_back(state, &log, ReadBack::Log)
			}
			// The entry committed and cold already moved past it, so the log no longer serves
			// it. Exactly one entry landed above the view in this epoch and claim entry, and only
			// this opening admits through its journal, so it is the unobserved admission: install
			// it from the payload this opening kept. Its rows stay in memory until a refresh
			// moves the snapshot past it.
			None if page.cold.volume_seq >= expected => {
				let payload = state
					.unobserved_admission
					.clone()
					.ok_or_else(|| corrupt!())?;
				let log = VolumeLogEntry {
					volume_seq: expected,
					client_epoch: state.client_epoch,
					written_at_unix_ms: 0,
					body: VolumeLogEntryBody::PayloadWrite(PayloadWrite {
						payload_family: rivet_data::families::VersionFamily::DEPOT_FS_ENTRY.0,
						payload_version: super::transport::embedded_version(&payload)?,
						client_seq: 0,
						request_id: [0; 16],
						payload_digest: [0; 16],
						payload: payload.into(),
						codec: PayloadCodec::None,
					}),
				};
				self.install_read_back(state, &log, ReadBack::Payload)
			}
			None => return Err(corrupt!()),
		};
		installed?;
		state.unobserved_admission = None;
		crate::metrics::FS_UNKNOWN_OUTCOMES
			.with_label_values(&["reconciled"])
			.inc();
		tracing::debug!(
			volume_seq = expected,
			client_epoch = state.client_epoch,
			"installed an admission with an unknown outcome from the log"
		);
		Ok(status)
	}

	/// Waits for the entries whose commit wait ended undecided to commit, bounded by
	/// `deadline`. Their layers are installed, and anything built on them is ordered after them
	/// in the same log, so once the volume client reports them committed the view is exact. A
	/// deadline that passes first returns `DeadlineExceeded` and keeps them undecided for the
	/// next request; latency never fences. A fence the barrier reports is a newer claim and
	/// fails the request. Costs one atomic load when nothing is undecided.
	async fn settle_undecided(&self, status: LogStatus, deadline: u64) -> Result<LogStatus> {
		let through = self.undecided_through.load(Ordering::Acquire);
		if through == 0 {
			return Ok(status);
		}
		let status = if status.committed >= through {
			status
		} else {
			// A request already past its deadline must not leave a barrier behind it.
			if self.now()?.boot_ms >= deadline {
				return Err(Error::DeadlineExceeded);
			}
			let ticket = self.journal.barrier()?;
			rivet_env::timeout_at(self.clock.as_ref(), deadline, ticket)
				.await
				.map_err(|_| Error::DeadlineExceeded)?
				.map_err(|error| {
					// Nothing behind a blocked entry can commit on this volume client, so
					// every later request would fail here; fence so the owner reopens.
					if matches!(
						error,
						depot_volume::volume_client_machine::types::WriteError::HistoryBlocked { .. }
					) {
						self.fence_with(crate::fences::FenceReason::HistoryBlocked);
					}
					Error::Log(error)
				})?;
			self.status()?
		};
		if status.committed >= through {
			// A newer undecided entry recorded meanwhile stays recorded.
			let _ = self.undecided_through.compare_exchange(
				through,
				0,
				Ordering::AcqRel,
				Ordering::Acquire,
			);
			crate::metrics::FS_UNKNOWN_OUTCOMES
				.with_label_values(&["settled"])
				.inc();
			Ok(status)
		} else {
			Err(Error::DeadlineExceeded)
		}
	}

	/// Installs one read-back entry of this opening's own epoch as its admission would have:
	/// its layer, its transaction progress and its conflict window change. Every fallible step
	/// runs before any state changes, so a failure leaves the entry unobserved for a retry.
	fn install_read_back(
		&self,
		state: &mut State,
		log: &VolumeLogEntry,
		source: ReadBack,
	) -> Result<()> {
		let seq = log.volume_seq;
		let entry = decode(log)?;
		let plan = apply::plan(seq, &entry)?;
		let bytes = super::admission::row_bytes(&plan);
		let layer = match source {
			ReadBack::Log => Layer::indexed(seq, plan, &self.budget, self.journal.clone())?,
			// The log no longer holds the entry, so its rows must stay in memory.
			ReadBack::Payload => {
				match Layer::charge_cached(&plan, &self.budget, self.config.max_cache_bytes) {
					Some(charge) => Layer::with_charge(seq, plan, charge),
					None => {
						self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
						return Err(Error::Fenced);
					}
				}
			}
		};
		let position_charge = self.budget.reserve(347)?;
		let now = self.now()?.boot_ms;
		let mut resolver = state.resolver.clone();
		let resolution = resolver.push(log)?;
		if source == ReadBack::Payload && !matches!(resolution, Resolution::Standalone(_)) {
			// A transaction entry read from memory would need its pending layers' rows too.
			self.fence_with(crate::fences::FenceReason::UnappliedLogEntries);
			return Err(Error::Fenced);
		}
		match resolution {
			Resolution::Standalone(standalone) => {
				let change = Change::new(seq, now, &standalone, &self.budget)?;
				state.conflicts.push(change, &self.config);
				self.push_layer(state, layer);
			}
			Resolution::Pending => {
				let Some(FsTxn::FsTxnPending(txn)) = &entry.txn else {
					return Err(corrupt!());
				};
				let pending = state
					.transactions
					.get_mut(txn.id.as_ref())
					.ok_or_else(|| self.fenced(crate::fences::FenceReason::TransactionMissing))?;
				pending.layers.push(layer);
				pending.add_bytes(bytes);
			}
			Resolution::Commit { id, open, .. } => {
				let pending = state
					.transactions
					.get(&id)
					.ok_or_else(|| self.fenced(crate::fences::FenceReason::TransactionMissing))?;
				let mut layers = pending.layers.clone();
				layers.push(layer);
				let merged = Layer::merge(seq, &layers, &self.budget)?;
				let mut footprint = open.footprint.clone();
				footprint.extend(depot_fs_core::txn::footprint(seq, &entry.ops));
				let change = Change::footprint(seq, now, &footprint, &self.budget)?;
				state.conflicts.push(change, &self.config);
				self.push_layer(state, merged);
				state.transactions.remove(&id);
				self.transaction_wake.notify_waiters();
			}
			Resolution::Discarded => {
				if let Some(txn) = &entry.txn {
					state.transactions.remove(depot_fs_core::txn::id(txn));
				}
				self.transaction_wake.notify_waiters();
			}
		}
		state.resolver = resolver;
		state.last_seq = seq;
		state
			.positions
			.insert(seq, (log.client_epoch, position_charge));
		Ok(())
	}
}

/// Where a read-back entry's bytes come from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadBack {
	/// The log still serves the entry, so its layer reads rows back from the log.
	Log,
	/// Cold passed the entry; it is rebuilt from the payload this opening admitted.
	Payload,
}

/// The entry's payload as it was admitted, before the volume client's payload codec.
fn raw_payload(log: &VolumeLogEntry) -> Result<Vec<u8>> {
	let VolumeLogEntryBody::PayloadWrite(payload) = &log.body else {
		return Err(corrupt!());
	};
	depot_types::payload_codec::decode(payload)
		.map(|raw| raw.to_vec())
		.map_err(|_| corrupt!())
}
fn decode(log: &VolumeLogEntry) -> Result<depot_fs_core::FsEntry> {
	let VolumeLogEntryBody::PayloadWrite(payload) = &log.body else {
		return Err(corrupt!());
	};
	let raw = depot_types::payload_codec::decode(payload).map_err(|_| corrupt!())?;
	DepotFsEntryData::deserialize_with_embedded_version(&raw).map_err(|_| corrupt!())
}
