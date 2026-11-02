//! Inputs from authorized callers and the serial storage executor.

use super::{
	state::recovery::{RecoveredShard, RecoveryError, RecoveryOutcome},
	types::{LogShardRequest, WriteToDiskError, WrittenToDisk},
};
use serde::{Deserialize, Serialize};

/// Inputs from callers, the supervisor, and the filesystem executor.
///
/// After handling an event, batch dispatch can arm `WriteToDisk` and emit
/// `WriteToDiskRequest`, release `Reply` effects, or emit `Stopped` after draining.
/// I/O completions also run segment maintenance while serving or draining; it can
/// emit `WriteCheckpointRequest` and `RemoveSegmentsRequest` with their timers.
/// Obsolete completions do not install their results. Failed or stopped shards
/// ignore events except requests, which receive an unavailable reply.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogShardEvent {
	/// **When**
	///
	/// The supervisor publishes a validated routing view through Shard::update_tablet_manifest.
	///
	/// **State**
	///
	/// Installs only newer manifest versions, queues removed-member retirement and re-admits clients whose requested
	/// version is now known. Later-version requests retain their original deadline and reservation.
	///
	/// **Effects**
	///
	/// Cancels the resumed requests’ ManifestRefresh timers. Re-admission can reject requests
	/// or produce frames. Retirement queues ForgetFrames through the same bounded write path;
	/// absence is published only after the disk barrier.
	TabletManifestUpdated {
		tablet_manifest: super::types::TabletManifestView,
	},

	/// **When**
	///
	/// `Shard::submit` and `Shard::submit_many` enqueue decoded commands, stamped with the
	/// submitting boot time.
	///
	/// **State**
	///
	/// While serving, a request that waited in the inbox past its bound changes nothing and
	/// is refused: `max_append_admission_wait_ms` for appends and commit announcements,
	/// `max_admission_wait_ms` for everything else. Admission applies the other commands to
	/// private replicas in order, then installs accepted memory state and adds frames and
	/// waiting replies to `memory_batches.filling`; capacity rejection leaves them unchanged.
	/// A future replica-set version reserves a bounded wait without changing a replica.
	///
	/// **Effects**
	///
	/// Each refusal replies `Overloaded` with a retry hint of the observed wait, clamped to
	/// `overload_retry_after_ms..=MAX_RETRY_AFTER_MS`. Waiting emits RefreshTabletManifest and
	/// arms ManifestRefresh. Admission failures reply immediately; accepted results wait for
	/// disk writes. Shared dispatch may emit `WriteToDiskRequest` and arm its `WriteToDisk`
	/// timer, or release frame-free replies.
	Submitted {
		submitted_boot_ms: u64,
		requests: Vec<LogShardRequest>,
	},

	/// **When**
	///
	/// A disk reader completes an admitted range read.
	///
	/// **State**
	///
	/// Removes the matching read and releases its reservations; a read that timed out or
	/// was dropped is ignored. An `Unavailable` error fails only this read: the entries a
	/// read scans are already synced, and writes keep their own fail-stop path. A `Corrupt`
	/// error fails the shard, because its indexed history is no longer trustworthy.
	///
	/// **Effects**
	///
	/// Cancels Read and delivers the page, replies `Overloaded` on `Unavailable`, or exits on
	/// `Corrupt`.
	ReadResponse {
		request_id: super::types::RequestId,
		result: Result<
			rivet_data::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse,
			super::types::DiskReadError,
		>,
	},

	/// **When**
	///
	/// The filesystem executor completes `WriteToDiskRequest`, after syncing bytes and any new
	/// segment name on success.
	///
	/// **State**
	///
	/// For the current write's last sequence, clears `memory_batches.writing_to_disk`, updates
	/// segment lengths, and replays frames into `disk_state`, removing replicas on forget
	/// frames. Failure sets `phase` to `Failed` and clears pending work.
	///
	/// **Effects**
	///
	/// Cancels the matching `WriteToDisk` timer and emits ordered `Reply` effects on success;
	/// failure emits runtime `Exit`. Shared maintenance and batch dispatch run afterward.
	WriteToDiskResponse {
		last_log_seq: u64,
		result: Result<WrittenToDisk, WriteToDiskError>,
	},

	/// **When**
	///
	/// The filesystem executor completes atomic checkpoint replacement.
	///
	/// **State**
	///
	/// For the current checkpoint boundary, installs coverage and returns the checkpoint
	/// cycle to `Idle`. Failure fails the shard.
	///
	/// **Effects**
	///
	/// Cancels the matching `WriteCheckpoint` timer. Shared maintenance may emit
	/// `RemoveSegmentsRequest` or another checkpoint write; batch dispatch follows. Failure
	/// emits runtime `Exit`.
	WriteCheckpointResponse {
		/// Installed base and delta file bytes after this replacement and any compaction.
		checkpoint_bytes: u64,
		last_included_log_seq: u64,
		result: Result<(), WriteToDiskError>,
	},

	/// **When**
	///
	/// The filesystem executor completes deletion of the selected files and syncs the directory
	/// on success.
	///
	/// **State**
	///
	/// Removes the outstanding segment entries from `disk_segments` and clears
	/// `disk_segments.removing`; failure fails the shard.
	///
	/// **Effects**
	///
	/// Cancels `RemoveSegments`, then runs shared maintenance and batch dispatch. Failure emits
	/// runtime `Exit`.
	RemoveSegmentsResponse {
		result: Result<(), WriteToDiskError>,
	},

	/// **When**
	///
	/// The startup filesystem scan finishes while `phase` is `Recovering(Scanning)`.
	///
	/// **State**
	///
	/// A valid clean image installs replicas, segment accounting, and sequence boundaries, then
	/// enters `Serving`. Checksum-detected corruption with complete tablet fences enters
	/// `Recovering(MarkingRepairing)` with fresh incarnations. Incomplete fences and every
	/// error, including a checksum-valid record that does not decode, fail the shard with the
	/// error's reason and leave the disk for an operator.
	///
	/// **Effects**
	///
	/// Cancels `Recover`. Corruption arms `MarkRepairing` and emits `MarkRepairingRequest`; failure emits
	/// runtime `Exit`. Successful recovery can dispatch maintenance.
	RecoverResponse(Result<RecoveryOutcome, RecoveryError>),

	/// **When**
	///
	/// The startup scan finished syncing or reading one more file while `phase` is
	/// `Recovering(Scanning)`.
	///
	/// **State**
	///
	/// None. A report after the scan ended is ignored.
	///
	/// **Effects**
	///
	/// Re-arms `Recover`, so the timer bounds the time without progress instead of the whole
	/// scan, which grows with the retained log. A disk that stops answering still fails the
	/// shard one `recovery_timeout_ms` after its last completed file.
	RecoverProgress,

	/// **When**
	///
	/// The executor finishes replacement writes, checkpoint installation, and quarantine while
	/// `phase` is `Recovering(MarkingRepairing)`.
	///
	/// **State**
	///
	/// Installs the rebuilt replicas and disk accounting, then enters `Serving`; an invalid
	/// image or I/O error fails the shard.
	///
	/// **Effects**
	///
	/// Cancels `MarkRepairing`, then allows shared maintenance and batch dispatch. Failure emits
	/// runtime `Exit`.
	MarkRepairingResponse(Result<RecoveredShard, WriteToDiskError>),

	/// **When**
	///
	/// `Shard::shutdown` requests shutdown after recovery settles; direct callers can also
	/// cancel startup with it.
	///
	/// **State**
	///
	/// Enters `Draining` until pending batches and maintenance finish, then `Stopped`. During
	/// recovery it enters `Stopped` immediately.
	///
	/// **Effects**
	///
	/// Startup cancellation cancels `Recover` and `MarkRepairing`. Batch dispatch finishes accepted
	/// writes and replies, then emits `Stopped` once idle.
	Drain,

	/// **When**
	///
	/// The executor finished a `NotifyMaintainerRequest`, delivered or not.
	///
	/// **State**
	///
	/// Frees the worker's notice slot when `notice_id` names the notice in flight to it; a
	/// response to an older notice, which its deadline already ended, changes nothing.
	/// Replicas changed after the notice was built stay marked. Any outcome but a full
	/// `Sent` marks the replicas it did not deliver again and holds the worker's next notice
	/// for a backoff that doubles with each consecutive failure; a full `Sent` clears the
	/// failure count.
	///
	/// **Effects**
	///
	/// Cancels `NoticeDeadline` for the worker. Dispatch may send the worker's next notice; a
	/// failure arms `NoticeRetry`. A tablet the worker does not run, or a refused notice,
	/// emits `RefreshTabletManifest`.
	NotifyMaintainerResponse {
		worker: rivet_topology::worker::WorkerId,
		notice_id: u64,
		outcome: super::types::NoticeOutcome,
	},
}

impl rivet_state_machine::EventBody for LogShardEvent {
	fn name(&self) -> &'static str {
		match self {
			LogShardEvent::TabletManifestUpdated { .. } => "tablet_manifest_updated",
			LogShardEvent::Submitted { .. } => "submitted",
			LogShardEvent::ReadResponse { .. } => "read_response",
			LogShardEvent::WriteToDiskResponse { .. } => "write_to_disk_response",
			LogShardEvent::WriteCheckpointResponse { .. } => "write_checkpoint_response",
			LogShardEvent::RemoveSegmentsResponse { .. } => "remove_segments_response",
			LogShardEvent::RecoverResponse(..) => "recover_response",
			LogShardEvent::RecoverProgress => "recover_progress",
			LogShardEvent::MarkRepairingResponse(..) => "mark_repairing_response",
			LogShardEvent::Drain => "drain",
			LogShardEvent::NotifyMaintainerResponse { .. } => "notify_maintainer_response",
		}
	}
}
