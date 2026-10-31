//! Transport requests refer to executor-owned payloads; replies go back to their caller.

use super::types::*;
use depot_types::VolumeLogKey;
use rivet_data::generated::shared_depot_types_v1::{ClientIdentity, VolumeLogPosition};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

/// Work emitted by volume client transitions and their shared dispatch pass.
///
/// `Payloads` executes transport requests through the injected `VolumeClientTransport`.
/// The driving owner must deliver caller replies and release payloads separately;
/// the production runner implements application replies, while recovery-only owners
/// handle repair completion. Executors return outcomes without changing volume client state.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolumeClientEffect {
	/// **When**
	///
	/// A recovery-only handle closes after publishing its claim entry, or its attempt fails.
	///
	/// **Executor**
	///
	/// The recovery executor resolves its owner's result with confirmed and incomplete
	/// members. The maintainer's repair job joins that owner, then delivers
	/// RecoverVolumeResponse to advance or retry the selected repair.
	RecoveryResponse {
		result: Result<RecoveryReport, WriteError>,
	},

	/// **When**
	///
	/// A commit barrier reaches its captured position, admission rejects it, its deadline
	/// expires, or the client is fenced.
	///
	/// **Executor**
	///
	/// The volume runner resolves the registered commit ticket with this result.
	/// Test drivers may record it directly; Payloads only handles transport work.
	WaitForCommitResponse {
		request_id: RequestId,
		result: Result<u64, WriteError>,
	},

	/// **When**
	///
	/// The current voting majorities freshly confirm the committed barrier, or admission, timeout, or fencing
	/// fails the wait.
	///
	/// **Executor**
	///
	/// The driving owner delivers the result to the confirmation caller. The current test
	/// driver records it; this effect is not sent through `VolumeClientTransport`.
	ConfirmResponse {
		request_id: RequestId,
		result: Result<u64, WriteError>,
	},

	/// **When**
	///
	/// Repair has matched the target tail selected by a claimed in-sync quorum.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::mark_in_sync` with the expected incarnation
	/// and tail, then returns `MatchResponse` with the same operation and worker IDs.
	MarkInSyncRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		replica_incarnation: [u8; 16],
		expected_appended: VolumeLogPosition,
	},

	/// **When**
	///
	/// Recovery needs another bounded page or divergence probe, or repair needs bytes absent
	/// from retained history.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::read`, stores returned bodies in `Payloads`,
	/// measures them, extracts retry identities, and returns handles and replica state in
	/// `ReadVolumeLogEntriesResponse`.
	ReadVolumeLogEntriesRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		start: u64,
		max_entries: u32,
		max_bytes: u32,
	},

	/// **When**
	///
	/// Matching finds a shared prefix, or removes the suffix above cold and marks it repairing.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::truncate` with the observed tail and
	/// retained boundary, then returns the correlated `MatchResponse`.
	TruncateRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		expected_appended: VolumeLogPosition,
		keep: VolumeLogPosition,
		mark_repairing: bool,
	},

	/// **When**
	///
	/// A confirmation needs fresh evidence, memory pressure needs a fresh cold position, or
	/// a peer failure prompts an acknowledged survivor to check the current membership. The
	/// committed position it carries is a free hint, as on appends: the maintainer derives
	/// commitment from the replica tails, so nothing sends this to publish progress.
	///
	/// **Executor**
	///
	/// Calls VolumeClientTransport::advance_committed and returns AdvanceCommittedResponse.
	/// The shard bounds the carried position by appended without matching.
	AdvanceCommittedRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		committed_volume_seq: u64,
	},

	/// **When**
	///
	/// The volume client starts opening, or a replica returns `VolumeNotLoaded` and old operations are
	/// retired before reopening.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::open` for the volume key and returns the
	/// maintainer row or error in `VolumeOpenResponse`. `create_if_absent` is set only while the
	/// identity is unresolved and the owner asked for it; the maintainer then creates a missing
	/// volume and grants this client's claim in the same open. The request executor reads the
	/// bodies of `initial` from the payload store, stamps each with its retry identity, and sends
	/// them as the create's initial writes.
	VolumeOpenRequest {
		deadline_boot_ms: u64,
		op_id: OpId,
		volume_key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		create_if_absent: bool,
		/// The creating open's first writes; empty on every other open.
		initial: Vec<super::state::InitialWrite>,
		/// Keys the creating open asks to read back; `None` on every other open.
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
	},

	/// **When**
	///
	/// Lifecycle starts a fresh epoch or recovery claims an unmatched peer.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::claim` with the client identity and target
	/// worker, then returns the correlated replica state or rejection in `ClaimResponse`.
	ClaimRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
	},

	/// **When**
	///
	/// The claim entry commits to a quorum, or opening/recovery fails and fences the handle.
	///
	/// **Executor**
	///
	/// The production volume runner publishes the open result to callers waiting for recovery.
	/// `Payloads::execute` does not perform reply delivery.
	OpenResponse {
		request_id: RequestId,
		result: Result<(), WriteError>,
	},

	/// **When**
	///
	/// Draining has no pending writes and a quorum has acknowledged the published commit
	/// position.
	///
	/// **Executor**
	///
	/// `Payloads::execute` calls `VolumeClientTransport::close` for the volume key and returns
	/// `VolumeCloseResponse`; it does not decide whether the volume client is safe to drain.
	VolumeCloseRequest {
		client: rivet_data::generated::shared_depot_types_v1::ClientIdentity,
		op_id: OpId,
		volume_log_key: VolumeLogKey,
	},

	/// **When**
	///
	/// An explicit close is rejected, finishes, times out, or is interrupted by fencing.
	///
	/// **Executor**
	///
	/// The production volume runner completes the identified close caller. No transport request
	/// is issued for this effect.
	CloseResponse {
		request_id: RequestId,
		result: Result<(), WriteError>,
	},

	/// **When**
	///
	/// A replica has capacity for more retained entries, repair copies a suffix, a confirmation
	/// bounded per-replica send window opens. Every append contains entries.
	///
	/// **Executor**
	///
	/// The owner calls `Payloads::prepare_append` synchronously in effect order, before
	/// another event or `ReleasePayload`. The resulting `PreparedAppend` owns its bytes;
	/// its future calls `VolumeClientTransport::append` and returns `AppendResponse`.
	AppendRequest {
		op_id: OpId,
		worker_id: WorkerId,
		volume_log_key: VolumeLogKey,
		client: ClientIdentity,
		tablet_replica_set_version: u64,
		previous: VolumeLogPosition,
		entries: Vec<EntryRef>,
		committed_volume_seq: u64,
	},

	/// **When**
	///
	/// Admission rejects or acknowledges an asynchronous write, a retry finds a committed
	/// receipt, a synchronous write commits, or timeout/fencing ends its caller wait.
	///
	/// **Executor**
	///
	/// The production volume runner delivers the result and any retry token to the write caller.
	/// `Payloads::execute` does not handle caller replies.
	WriteResponse {
		request_id: RequestId,
		result: Result<WriteOutcome, WriteError>,
	},

	/// **When**
	///
	/// Admission discards a rejected or duplicate submission, or recovery/repair discards
	/// obsolete, invalid, duplicate, or superseded bytes; cold progress also evicts retained payloads.
	///
	/// **Executor**
	///
	/// The driving owner calls `Payloads::release` in effect order. Previously prepared
	/// appends own their bytes and remain valid. Read responses must already be materialized.
	/// The production volume runner implements this contract; no response event is produced.
	ReleasePayload { payload: PayloadId },
}

impl rivet_state_machine::Effect for VolumeClientEffect {
	fn name(&self) -> &'static str {
		match self {
			VolumeClientEffect::RecoveryResponse { .. } => "recovery_response",
			VolumeClientEffect::WaitForCommitResponse { .. } => "wait_for_commit_response",
			VolumeClientEffect::ConfirmResponse { .. } => "confirm_response",
			VolumeClientEffect::MarkInSyncRequest { .. } => "mark_in_sync_request",
			VolumeClientEffect::ReadVolumeLogEntriesRequest { .. } => {
				"read_volume_log_entries_request"
			}
			VolumeClientEffect::TruncateRequest { .. } => "truncate_request",
			VolumeClientEffect::AdvanceCommittedRequest { .. } => "advance_committed_request",
			VolumeClientEffect::VolumeOpenRequest { .. } => "volume_open_request",
			VolumeClientEffect::ClaimRequest { .. } => "claim_request",
			VolumeClientEffect::OpenResponse { .. } => "open_response",
			VolumeClientEffect::VolumeCloseRequest { .. } => "volume_close_request",
			VolumeClientEffect::CloseResponse { .. } => "close_response",
			VolumeClientEffect::AppendRequest { .. } => "append_request",
			VolumeClientEffect::WriteResponse { .. } => "write_response",
			VolumeClientEffect::ReleasePayload { .. } => "release_payload",
		}
	}
}
