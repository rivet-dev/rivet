//! Write outcomes and journal checks. A writer never reopens itself: a volume client that
//! loses its claim or its quorum fails, and the filesystem fails with it, so its owner opens
//! the volume again with a new claim. A write whose outcome is still unknown at its deadline
//! is latency, not a lost claim: the caller gets the unknown outcome with its retry token and
//! the filesystem keeps serving, because the volume client keeps the entry until it commits.
//! Plain reads only see committed entries, and every later request first waits, within its own
//! deadline, for the undecided entry to commit (`Filesystem::undecided_through`), so no read,
//! transaction or write observes or builds on it before then. If a newer claim fences the
//! volume client first, the entry is lost with every later one, none of which was
//! acknowledged, and this filesystem fences through its journal status.
use super::{
	Error, Filesystem, Result,
	client::{Attempt, State},
	transport::LogStatus,
};
use depot_volume::volume_client_machine::{
	runner::CommitTicket,
	types::{ErrorDisposition, WriteError, WriteOutcome, WriteToken},
};
use std::sync::atomic::Ordering;

pub(super) struct Receipt<'a> {
	pub outcome: WriteOutcome,
	pub payload: vbare::Bytes,
	pub ticket: Option<Result<CommitTicket>>,
	pub attempt: Attempt<'a>,
}



impl Filesystem {
	pub(super) async fn pinned_result<T>(&self, result: Result<T>, deadline: u64) -> Result<T> {
		if matches!(&result, Err(Error::TooOld)) && !self.fenced.load(Ordering::Acquire) {
			// Cold can pass a pinned view's sequence, and trimming can invalidate an indexed
			// layer. Learning the newer cold position lets the next refresh move the floor past
			// it without revoking other valid pins.
			let confirmed =
				rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.confirm())
					.await
					.map_err(|_| Error::DeadlineExceeded)?;
			confirmed?;
			return result;
		}
		result
	}

	pub(crate) async fn confirmed_minimum(&self) -> Result<depot_fs_core::FsForkMinimum> {
		let deadline = self.deadline()?;
		let _write = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.write_gate.lock())
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
		self.recover_before(deadline).await?;
		let seq = rivet_env::timeout_at(self.clock.as_ref(), deadline, self.journal.confirm())
			.await
			.map_err(|_| Error::DeadlineExceeded)??;
		self.fork_minimum(seq).await
	}

	/// Whether an admitted write's commit is still undecided on the volume client that
	/// admitted it: its quorum has not answered yet, or the commit wait was refused.
	fn outcome_pending<T>(&self, result: &Result<T>) -> bool {
		!self.fenced.load(Ordering::Acquire)
			&& self.journal.status().is_ok()
			&& matches!(
				result,
				Err(Error::Overloaded
					| Error::Log(
						WriteError::QuorumUnavailable { .. }
							| WriteError::DeadlineExceeded { .. }
							| WriteError::Overloaded { .. }
					))
			)
	}

	pub(super) async fn recover_before(&self, deadline: u64) -> Result<()> {
		let mut state = self.state.lock().await;
		self.current(&mut state, deadline).await.map(|_| ())
	}
	/// The journal's status. The journal never changes volume client under this filesystem,
	/// so a status it still reports is the history this filesystem replayed; the filesystem
	/// never rebuilds itself on a new client.
	pub(super) async fn current(&self, _state: &mut State, _deadline: u64) -> Result<LogStatus> {
		self.status()
	}
	/// A refusal of a barrier or recovery read cannot prove that an admitted write did not run.
	pub(super) fn admitted_error(error: Error, token: WriteToken) -> Error {
		match error {
			Error::DeadlineExceeded | Error::Log(WriteError::DeadlineExceeded { token: None }) => {
				Error::Log(WriteError::DeadlineExceeded { token: Some(token) })
			}
			Error::Overloaded
			| Error::Unavailable
			| Error::ColdReadRefused
			| Error::Lagging
			| Error::TooOld
			| Error::Conflict
			| Error::Busy => Error::Log(WriteError::QuorumUnavailable { token: Some(token) }),
			Error::Log(ref error) if error.disposition() == ErrorDisposition::NotExecuted => {
				Error::Log(WriteError::QuorumUnavailable { token: Some(token) })
			}
			error => error,
		}
	}
	pub(super) async fn finish_write(
		&self,
		receipt: Receipt<'_>,
		deadline: u64,
	) -> Result<WriteOutcome> {
		let token = receipt.outcome.token.clone();
		self.finish_admitted(receipt, deadline)
			.await
			.map_err(|error| Self::admitted_error(error, token))
	}
	/// Waits for an admitted entry to commit. Success and a decided failure behave as before:
	/// a fence or a broken client leaves the attempt armed, so the filesystem fences. An
	/// outcome still unknown at the deadline while the same history serves disarms it: the
	/// entry's layer is already installed and the volume client keeps the entry until it
	/// commits, or fences on a newer claim, which fences this filesystem through its status.
	async fn finish_admitted(&self, receipt: Receipt<'_>, deadline: u64) -> Result<WriteOutcome> {
		let Receipt {
			outcome,
			payload,
			ticket,
			attempt,
		} = receipt;
		let seq = outcome.volume_seq;
		let mut join_decided = false;
		let result = self
			.await_commit(outcome, payload, ticket, deadline, &mut join_decided)
			.await;
		self.settle_commit(attempt, seq, result, join_decided)
	}

	/// Ends an admitted entry's attempt after its commit wait. Success finishes it. A wait that
	/// ended undecided while the journal still serves finishes it too and records `seq` as
	/// undecided: the volume client keeps the entry until it commits, or fences on a newer
	/// claim, which fences this filesystem through its status, and later requests wait for it
	/// first. Any decided failure leaves the attempt armed, so it fences.
	pub(super) fn settle_commit<T>(
		&self,
		attempt: Attempt<'_>,
		seq: u64,
		result: Result<T>,
		decided: bool,
	) -> Result<T> {
		match result {
			Ok(value) => {
				attempt.finish();
				Ok(value)
			}
			Err(error)
				if !decided
					&& !self.fenced.load(Ordering::Acquire)
					&& Self::commit_undecided(&error)
					&& self.status().is_ok() =>
			{
				crate::metrics::FS_UNKNOWN_OUTCOMES
					.with_label_values(&["commit"])
					.inc();
				self.undecided_through.fetch_max(seq, Ordering::AcqRel);
				attempt.finish();
				Err(error)
			}
			Err(error) => Err(error),
		}
	}

	/// Whether a commit wait ended without deciding its entry: its deadline passed, its quorum
	/// had not answered, or the wait was refused under load.
	pub(super) fn commit_undecided(error: &Error) -> bool {
		matches!(
			error,
			Error::DeadlineExceeded
				| Error::Overloaded
				| Error::Log(
					WriteError::QuorumUnavailable { .. }
						| WriteError::DeadlineExceeded { .. }
						| WriteError::Overloaded { .. }
				)
		)
	}

	/// Waits for the commit, joining the entry through its retry identity while the outcome
	/// stays pending. Sets `join_decided` when a join answered with anything but a pending
	/// outcome, such as a fence or a retry conflict; the error returned is then still the
	/// commit wait's own, which keeps the entry's uncertainty for the caller.
	async fn await_commit(
		&self,
		mut outcome: WriteOutcome,
		payload: vbare::Bytes,
		ticket: Option<Result<CommitTicket>>,
		deadline: u64,
		join_decided: &mut bool,
	) -> Result<WriteOutcome> {
		let mut result = match ticket.ok_or_else(|| corrupt!())? {
			Ok(ticket) => rivet_env::timeout_at(self.clock.as_ref(), deadline, ticket)
				.await
				.map_err(|_| Error::DeadlineExceeded)?
				.map_err(Error::Log),
			Err(error) => Err(error),
		};
		// The entry is admitted and the same volume client keeps replicating it, so a quorum
		// that has not answered yet, or a refused commit wait, leaves the outcome open rather
		// than failed. Wait for it until this caller's deadline by joining the entry through
		// its retry identity. A deadline that passes with the outcome still unknown returns it
		// to the caller, and the entry commits later in its place. A wait the volume client
		// released because a quorum of shards refuses for a full disk returns at once with the
		// shards' hint: space returns only after cold writes and trims, so joining again would
		// only hold the caller to its deadline.
		while self.outcome_pending(&result) && !released_for_full_disk(&result) {
			// Start watching for progress before the join, so a change between a refusal and
			// the wait still wakes it.
			let progress = self.journal.wait_progress();
			let joined = rivet_env::timeout_at(
				self.clock.as_ref(),
				deadline,
				self.journal.retry(payload.clone(), outcome.token.clone()),
			)
			.await
			.map_err(|_| Error::DeadlineExceeded)?;
			match joined {
				// The retry joined the original entry, which committed where the view holds
				// it, so every entry before it committed too: the view is exact.
				Ok(joined) if joined.applied_earlier && joined.volume_seq == outcome.volume_seq => {
					return Ok(joined);
				}
				// A volume client never drops an entry it admitted: it commits it or fails. A
				// join that appended the write again therefore means the client broke that rule.
				Ok(_) => return Err(corrupt!()),
				Err(error) => {
					// A join answered with anything but a pending outcome, such as a fence or a
					// retry conflict, ends the wait and fences this filesystem.
					if !self.outcome_pending::<()>(&Err(error.clone())) {
						tracing::debug!(?error, "joining an admitted write failed");
						*join_decided = true;
						break;
					}
					// A refused join returns at once: wait for the volume client to make
					// progress before joining again.
					if matches!(
						error,
						Error::Overloaded | Error::Log(WriteError::Overloaded { .. })
					) {
						let progressed =
							rivet_env::timeout_at(self.clock.as_ref(), deadline, progress)
								.await
								.map_err(|_| Error::DeadlineExceeded)?;
						if let Err(error) = progressed {
							tracing::debug!(?error, "volume client progress is unavailable");
							break;
						}
					}
					result = Err(error);
				}
			}
		}
		let through = result?;
		outcome.committed = Some(through);
		Ok(outcome)
	}
}

/// Whether the volume client released a commit wait with its write's token because a quorum
/// of shards refuses for a full disk.
fn released_for_full_disk<T>(result: &Result<T>) -> bool {
	matches!(
		result,
		Err(Error::Log(WriteError::Overloaded { token: Some(_), .. }))
	)
}
