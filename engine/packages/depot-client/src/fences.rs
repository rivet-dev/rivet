//! Why a filesystem, its journal or its volume client was fenced.
//!
//! Every site that fences, or that refuses work because its view is fenced, names a reason.
//! Each one counts in `depot_client_fences_total` by reason and logs once per volume owner,
//! with the volume key and client epoch, so the first cause of a fence is visible.

use depot_types::VolumeLogKey;
use std::sync::atomic::{AtomicBool, Ordering};

/// A code-defined fence cause. Labels are bounded; volume keys never become labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceReason {
	/// A mutation attempt was dropped before it finished, so its outcome is unknown.
	AttemptAbandoned,
	/// The commit queue driver was dropped with commits still queued.
	CommitDriverAbandoned,
	/// The log's next sequence is not the one this view expects at admission.
	AdmitSequenceMismatch,
	/// A staged format entry no longer lines up with the log's next sequence.
	StagedFormatMismatch,
	/// A volume client reopen took the sequence this write expected.
	AdmitReopened,
	/// The log holds entries this opening never applied.
	UnappliedLogEntries,
	/// A transaction this request names is no longer pending.
	TransactionMissing,
	/// Aborting an expired or dropped transaction failed.
	TransactionAbortFailed,
	/// An open transaction survived replay into a new client epoch.
	ReplayOpenTransaction,
	/// The journal now names a different volume.
	KeyMismatch,
	/// History or rows were corrupt.
	Corrupt,
	/// Replaying the volume's history into a new opening failed.
	ReplayFailed,
	/// Cold names a client epoch this view never applied.
	ColdEpochMismatch,
	/// A refreshed cold snapshot is not at the journal's cold position.
	ColdRefreshMismatch,
	/// The opening cold snapshot does not match the recovered history.
	OpenColdMismatch,
	/// A fork minimum names a sequence that is not committed.
	ForkMinimumUncommitted,
	/// A remote read session's view moved under it.
	RemoteReadStale,
	/// The maintainer answered a filesystem read as fenced for this client epoch.
	RemoteReadFenced,
	/// The log shards permanently refuse an entry in this opening's history, so nothing
	/// behind it can commit on this volume client; the owner reopens to move past it.
	HistoryBlocked,
	/// The filesystem was already fenced.
	AlreadyFenced,
	/// The journal was already fenced, or its caller may no longer serve.
	JournalFenced,
	/// A replica rejected the volume client for a newer client, so it lost the volume.
	VolumeClientFenced,
	/// This process fenced, closed or shut down the volume client.
	VolumeClientClosed,
	/// The volume client has not resolved its volume yet.
	VolumeUnresolved,
	/// A capture volume was already created.
	CaptureCreated,
	/// The owning slot, lease or filesystem handle was released.
	OwnerReleased,
	/// The slot replaced this filesystem's volume client, or holds none for a reopen.
	SlotReplaced,
	/// The slot closed this filesystem's volume client.
	SlotClosed,
	/// The volume was deleted while this filesystem held it.
	VolumeDeleted,
	/// The journal's owner released the volume, such as an owner that retired or a released
	/// slot. Terminal for this journal, but not evidence that another client writes the volume.
	DispatchRevoked,
	/// The owner of a staged write failed a request and could not abort the staged transaction.
	StagedWriteAbortFailed,
	/// The filesystem's owner retired, or the filesystem was installed after it ended.
	OwnerRetired,
	/// A command volume could not refresh its cold view.
	CommandColdTooOld,
	/// Not a fence: the volume client was reopening itself when a request checked it.
	VolumeClientNotServing,
}

impl FenceReason {
	pub fn label(self) -> &'static str {
		match self {
			FenceReason::AttemptAbandoned => "attempt_abandoned",
			FenceReason::CommitDriverAbandoned => "commit_driver_abandoned",
			FenceReason::AdmitSequenceMismatch => "admit_sequence_mismatch",
			FenceReason::StagedFormatMismatch => "staged_format_mismatch",
			FenceReason::AdmitReopened => "admit_reopened",
			FenceReason::UnappliedLogEntries => "unapplied_log_entries",
			FenceReason::TransactionMissing => "transaction_missing",
			FenceReason::TransactionAbortFailed => "transaction_abort_failed",
			FenceReason::ReplayOpenTransaction => "replay_open_transaction",
			FenceReason::KeyMismatch => "key_mismatch",
			FenceReason::Corrupt => "corrupt",
			FenceReason::ColdEpochMismatch => "cold_epoch_mismatch",
			FenceReason::ColdRefreshMismatch => "cold_refresh_mismatch",
			FenceReason::OpenColdMismatch => "open_cold_mismatch",
			FenceReason::ForkMinimumUncommitted => "fork_minimum_uncommitted",
			FenceReason::RemoteReadStale => "remote_read_stale",
			FenceReason::RemoteReadFenced => "remote_read_fenced",
			FenceReason::HistoryBlocked => "history_blocked",
			FenceReason::AlreadyFenced => "already_fenced",
			FenceReason::JournalFenced => "journal_fenced",
			FenceReason::VolumeClientFenced => "volume_client_fenced",
			FenceReason::VolumeClientClosed => "volume_client_closed",
			FenceReason::VolumeUnresolved => "volume_unresolved",
			FenceReason::CaptureCreated => "capture_created",
			FenceReason::OwnerReleased => "owner_released",
			FenceReason::ReplayFailed => "replay_failed",
			FenceReason::SlotReplaced => "slot_replaced",
			FenceReason::SlotClosed => "slot_closed",
			FenceReason::VolumeDeleted => "volume_deleted",
			FenceReason::DispatchRevoked => "dispatch_revoked",
			FenceReason::StagedWriteAbortFailed => "staged_write_abort_failed",
			FenceReason::OwnerRetired => "owner_retired",
			FenceReason::CommandColdTooOld => "command_cold_too_old",
			FenceReason::VolumeClientNotServing => "volume_client_not_serving",
		}
	}

	/// The owner ended this lifetime itself: it dropped or closed its handle, or retired, as
	/// every actor stop and idle release does. These are logged at debug; they still count by
	/// reason and still take the once-per-owner line, so nothing later repeats it.
	fn routine(self) -> bool {
		matches!(
			self,
			FenceReason::OwnerReleased | FenceReason::SlotClosed | FenceReason::OwnerRetired
		)
	}

	/// Consequences of an earlier fence rather than a new cause. They count but never take
	/// the once-per-volume log line, which belongs to the first cause.
	fn consequence(self) -> bool {
		matches!(
			self,
			FenceReason::AlreadyFenced | FenceReason::JournalFenced
		)
	}
}

/// Counts one fence, and logs the first cause for this owner, marked in `logged`.
pub(crate) fn record(
	reason: FenceReason,
	key: Option<&VolumeLogKey>,
	client_epoch: Option<u64>,
	logged: &AtomicBool,
) {
	record_with(reason, key, client_epoch, None, logged);
}

/// Like `record`, with the volume client phase that explains a refusal.
pub(crate) fn record_with(
	reason: FenceReason,
	key: Option<&VolumeLogKey>,
	client_epoch: Option<u64>,
	phase: Option<&dyn std::fmt::Debug>,
	logged: &AtomicBool,
) {
	count(reason);
	if reason.consequence() || logged.swap(true, Ordering::AcqRel) {
		return;
	}
	if reason.routine() {
		tracing::debug!(
			volume_key = ?key.map(|key| &key.0.volume_key),
			volume_incarnation = ?key.map(|key| key.0.volume_incarnation),
			?client_epoch,
			reason = reason.label(),
			"depot filesystem released by its owner"
		);
		return;
	}
	tracing::warn!(
		volume_key = ?key.map(|key| &key.0.volume_key),
		volume_incarnation = ?key.map(|key| key.0.volume_incarnation),
		?client_epoch,
		?phase,
		reason = reason.label(),
		"depot filesystem fenced; its owner must reopen the volume"
	);
}

/// Counts one fence without logging, for sites with no volume owner to log once for.
pub(crate) fn count(reason: FenceReason) {
	crate::metrics::FENCES
		.with_label_values(&[reason.label()])
		.inc();
}
