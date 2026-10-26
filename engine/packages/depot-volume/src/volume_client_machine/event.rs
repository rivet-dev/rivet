//! Caller submissions and correlated transport completions.

use super::types::*;
use depot_types::payload_codec::PayloadDigest;
use rivet_data::generated::{
	depot_log_shard_v1::VolumeLogReplicaState, depot_storage_worker_protocol_v1::RejectResponse,
};
use rivet_topology::worker::WorkerId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RpcError {
	/// The maintainer rejected this lifecycle command before returning a row.
	LifecycleRejected(rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse),
	/// No cached maintainer route was available; this attempt sent no command.
	RoutingUnavailable,
	Rejected(RejectResponse),
	/// The transport refused the request before it reached the replica, such as when this
	/// process's queue to the worker is full. Nothing was sent, so it is no evidence about the
	/// replica's log: the caller waits `retry_after_ms` like a shard overload refusal and never
	/// starts repair for it.
	Overloaded {
		retry_after_ms: u64,
	},
	Unavailable,
}

/// Caller inputs and executor completions, correlated by request or operation identity.
///
/// After each event, shared dispatch advances replica matching, prepares the claim entry,
/// fills append windows, completes barriers, and checks draining. Those steps may
/// emit `ClaimRequest`, `ReadVolumeLogEntriesRequest`, `TruncateRequest`,
/// `MarkInSyncRequest`, `AppendRequest`, barrier replies, or `VolumeCloseRequest`,
/// and arm their corresponding timers. Obsolete completions do not install evidence.
/// A current `StaleTabletReplicaSet` reply adopts a newer configuration, retires old
/// operation IDs and confirmation votes, and resends with unchanged entry identities.
/// Recovery before the claim entry rechecks its claiming quorums at the same client epoch;
/// an active client keeps serving while it matches any new destination.
/// Fencing retires outstanding timers and fails pending write, barrier, and lifecycle
/// callers; their reply effects do not imply that previously sent writes were undone.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolumeClientEvent {
	/// **When**
	///
	/// An application explicitly waits for readiness, without submitting a read or write.
	///
	/// **State**
	///
	/// Registers a bounded caller during opening; serving or terminal clients answer immediately.
	///
	/// **Effects**
	///
	/// Arms the caller's original deadline or emits its correlated OpenResponse.
	OpenRequest { request_id: RequestId },
	/// **When**
	///
	/// The owner loses its fixed authority, or its executor cannot safely complete a request.
	///
	/// **State**
	///
	/// Fences this handle and retires pending operation evidence. Repeated fencing cannot revive it.
	///
	/// **Effects**
	///
	/// Cancels timers, releases pending callers and reports failure to the recovery owner if one is waiting.
	Fence,

	/// **When**
	///
	/// The owner's manifest view names a newer tablet replica set than this client uses. An
	/// idle client sends nothing a replica could reject, so this is how it learns of a
	/// membership change.
	///
	/// **State**
	///
	/// Adopts the set as a `StaleTabletReplicaSet` rejection would, keeping its claim epoch.
	/// An older or equal set, or a client that is not serving or recovering, ignores it.
	///
	/// **Effects**
	///
	/// Retires in-flight appends and repair of removed members; shared dispatch then matches
	/// the new members.
	ReplicaSetObserved {
		replica_set: rivet_data::generated::shared_depot_types_v1::TabletReplicaSet,
	},

	/// **When**
	///
	/// A caller wants all writes admitted before this request to reach an in-sync quorum.
	///
	/// **State**
	///
	/// Stores a barrier at `next_volume_seq - 1`; removes it once `committed` reaches that
	/// position. A supplied history must match the current epoch and claim entry.
	/// Stale-history, non-serving or over-budget requests store no barrier, nor does one
	/// while a quorum is blocked at an entry, which is rejected with `HistoryBlocked`.
	///
	/// **Effects**
	///
	/// Arms a caller `Quorum` timer. Emits `WaitForCommitResponse` on completion or rejection
	/// and cancels the timer when completed.
	WaitForCommitRequest {
		request_id: RequestId,
		history: Option<HistoryIdentity>,
	},

	/// **When**
	///
	/// A caller wants fresh quorum evidence for the tail captured by this request.
	///
	/// **State**
	///
	/// Stores a confirmation barrier and accumulates distinct responding workers after its
	/// position commits; removes it after fresh confirmations from every required majority.
	///
	/// **Effects**
	///
	/// Arms `Quorum`, dispatches empty `AppendRequest` effects with `Append` timers, then emits
	/// `ConfirmResponse` and cancels `Quorum`; rejected requests reply immediately.
	ConfirmRequest { request_id: RequestId },

	/// **When**
	///
	/// `Payloads::execute` finishes a recovery page, divergence probe, or repair fetch and the
	/// driving owner delivers the result.
	///
	/// **State**
	///
	/// A valid replay page updates `retained`, retry receipts, memory accounting, and replay
	/// progress. Probes update repair search bounds; fetches fill missing retained history.
	/// Invalid evidence retries or fences recovery. A fetch the source refuses as too large
	/// cannot page even its first entry within this client's read bound, so the repair
	/// target is blocked at that entry instead of fetching it again.
	///
	/// **Effects**
	///
	/// Cancels the matching `Control` timer. May emit another read, `TruncateRequest`,
	/// or repair `AppendRequest` with a new timer. Unused handles emit
	/// `ReleasePayload`; stale pages only release handles. Shared dispatch follows.
	ReadVolumeLogEntriesResponse {
		op_id: OpId,
		worker_id: WorkerId,
		result: Result<(VolumeLogReplicaState, Vec<EntryRef>), RpcError>,
	},

	/// **When**
	///
	/// The transport executor completes `TruncateRequest`, or
	/// `MarkInSyncRequest`.
	///
	/// **State**
	///
	/// Matching validates the pending operation, expected tail, claim, and incarnation; success
	/// updates replica evidence and advances repair or marks it matched. Failure backs off,
	/// reopens, or fences the client.
	///
	/// **Effects**
	///
	/// Cancels the matching `Control` timer; continued repair can emit reads, appends, or peer
	/// claims with timers. Failure can arm repair backoff, emit `VolumeOpenRequest`, or emit
	/// failure replies. Shared dispatch follows.
	MatchResponse {
		op_id: OpId,
		worker_id: WorkerId,
		result: Result<VolumeLogReplicaState, RpcError>,
	},

	/// **When**
	///
	/// The transport executor returns the maintainer result for the current
	/// `VolumeOpenRequest`.
	///
	/// **State**
	///
	/// Validates the volume lifetime and cold floor, advances `cold`, `committed`, and the
	/// epoch floor, then enters `Recovering(Claiming)` with a fresh client epoch. A serving
	/// open the maintainer refused as overloaded, splitting or not its maintainer, or that
	/// found no route or no answer, fences the handle with `WriteError::Refused` and a hint;
	/// the edge owns the retry. Any other failed or invalid open fences the handle.
	///
	/// **Effects**
	///
	/// Cancels the open `Control` timer and emits `ClaimRequest` with new `Control` timers for
	/// each replica. Failures emit `OpenResponse` and fail any outstanding callers.
	VolumeOpenResponse {
		op_id: OpId,
		result:
			Result<rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse, RpcError>,
	},

	/// **When**
	///
	/// The transport executor returns a claim attempt from a current or obsolete round.
	///
	/// **State**
	///
	/// Matching replies update claim evidence, incarnations, and the observed epoch. An in-sync
	/// quorum selects replay history; repairing replies never count toward that quorum.
	/// A newer claim fences the handle; the client never claims again on its own.
	///
	/// **Effects**
	///
	/// Cancels the matching `Control` timer. May emit replay reads,
	/// or `MarkInSyncRequest` with timers; a volume no longer loaded on a quorum and fencing
	/// emit failure replies. Shared dispatch follows.
	ClaimResponse {
		op_id: OpId,
		worker_id: WorkerId,
		result: Result<VolumeLogReplicaState, RpcError>,
	},

	/// **When**
	///
	/// A caller explicitly closes a serving volume client.
	///
	/// **State**
	///
	/// Changes `Appending` to `Draining` and records the caller. Once writes commit and a
	/// quorum knows the commit position, enters `Closing` with a release operation. Requests in
	/// other phases are rejected.
	///
	/// **Effects**
	///
	/// Shared dispatch continues replication and barriers. Draining cancels replica-operation
	/// timers, then emits `VolumeCloseRequest` with a `Control` timer. Rejection emits
	/// `CloseResponse`.
	CloseRequest { request_id: RequestId },

	/// **When**
	///
	/// The transport executor finishes the current maintainer release.
	///
	/// **State**
	///
	/// Changes the matching `Closing` attempt to `Stopped`, whether release succeeded or
	/// failed; obsolete responses do not change the close state.
	///
	/// **Effects**
	///
	/// Cancels the release `Control` timer. Emits `CloseResponse` when an
	/// explicit caller is waiting.
	VolumeCloseResponse {
		op_id: OpId,
		result: Result<(), RpcError>,
	},

	/// **When**
	///
	/// A caller registers bytes with `Payloads`, then submits the handle, block-address projection, digest,
	/// measured size, and optional retry token.
	///
	/// **State**
	///
	/// A new admitted mutation advances sequence counters and populates `retained`, `writes`,
	/// retry receipts and memory accounting. A known retry joins its existing entry or returns
	/// its receipt; rejection retains no new payload. An entry larger than a quorum's learned
	/// append limit is rejected with `EntryTooLarge`, and any write while a quorum is blocked
	/// at an entry with `HistoryBlocked`; neither takes a position.
	///
	/// **Effects**
	///
	/// Admitted synchronous callers arm `Quorum`, and new outstanding work arms
	/// `Stall`. Admission or retry results emit `WriteResponse`; unused bytes emit
	/// `ReleasePayload`. Memory pressure sends bounded `AdvanceCommittedRequest` probes
	/// to idle peers. Shared dispatch sends `AppendRequest` with `Append` timers.
	WriteRequest {
		request_id: RequestId,
		payload: PayloadId,
		payload_digest: PayloadDigest,

		retry_of: Option<WriteToken>,
		encoded_bytes: u32,
		sync: bool,
	},

	/// **When**
	///
	/// The transport executor returns disk write evidence or failure for an append, empty
	/// confirmation, or repair copy.
	///
	/// **State**
	///
	/// Normal replies retire the matching in-flight operation and advance replica
	/// acknowledgments, published commit progress, `committed`, and barrier evidence. A
	/// committed claim entry enters `Appending`; repair replies advance matching instead. A reply
	/// to an append already retired at its timeout still advances the acknowledgment of a
	/// matched replica claimed by this client. Failures back off, or fence on a newer claim.
	/// A `RequestTooLarge` refusal lowers the replica's append limit and resends smaller
	/// batches; an entry the shard can never take, for size or payload version, blocks the
	/// replica at it without repair, new claim or fence.
	///
	/// **Effects**
	///
	/// Cancels `Append`, or `Control` for repair. Commit progress updates `Stall` and emits
	/// completed `WriteResponse` or `OpenResponse`; failure may emit `VolumeOpenRequest` or
	/// failure replies. A blocked quorum fails the writes and barriers behind it with
	/// `HistoryBlocked` and cancels `Stall`. Shared dispatch can send more appends and
	/// release barriers.
	AppendResponse {
		op_id: OpId,
		worker_id: WorkerId,
		result: Result<VolumeLogReplicaState, RpcError>,
	},

	/// **When**
	///
	/// The executor returns a membership probe, capacity probe, or confirmation result.
	///
	/// **State**
	///
	/// Accepts only the current operation and incarnation, updates the observed cold
	/// position, and completes confirmation barriers. Obsolete replies are ignored.
	///
	/// **Effects**
	///
	/// Cancels the request timeout, may reply to barriers or close, and fences on lost
	/// authority; transient failures schedule replica repair. Valid cold progress can emit
	/// `ReleasePayload`, making retained memory available to later writes.
	AdvanceCommittedResponse {
		op_id: OpId,
		worker_id: WorkerId,
		result: Result<VolumeLogReplicaState, RpcError>,
	},
}

impl rivet_state_machine::EventBody for VolumeClientEvent {
	fn name(&self) -> &'static str {
		match self {
			VolumeClientEvent::OpenRequest { .. } => "open_request",
			VolumeClientEvent::Fence => "fence",
			VolumeClientEvent::ReplicaSetObserved { .. } => "replica_set_observed",
			VolumeClientEvent::WaitForCommitRequest { .. } => "wait_for_commit_request",
			VolumeClientEvent::ConfirmRequest { .. } => "confirm_request",
			VolumeClientEvent::ReadVolumeLogEntriesResponse { .. } => {
				"read_volume_log_entries_response"
			}
			VolumeClientEvent::MatchResponse { .. } => "match_response",
			VolumeClientEvent::VolumeOpenResponse { .. } => "volume_open_response",
			VolumeClientEvent::ClaimResponse { .. } => "claim_response",
			VolumeClientEvent::CloseRequest { .. } => "close_request",
			VolumeClientEvent::VolumeCloseResponse { .. } => "volume_close_response",
			VolumeClientEvent::WriteRequest { .. } => "write_request",
			VolumeClientEvent::AppendResponse { .. } => "append_response",
			VolumeClientEvent::AdvanceCommittedResponse { .. } => "advance_committed_response",
		}
	}
}
