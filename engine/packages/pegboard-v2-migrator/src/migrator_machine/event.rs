//! Inputs of the migrator state machine. Responses echo the `OpId` of the effect they answer;
//! a response whose op is not the one the state waits on is stale and changes nothing.
use rivet_util::Id;
use serde::{Deserialize, Serialize};

use super::types::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	/// **When**
	///
	/// The executor finished `LoadStateRequest`: the tenure claimed the state volume.
	///
	/// **State**
	///
	/// `Loaded` moves to `Running` and rebuilds the entries by the failover rule: a copying entry is
	/// discarded, a confirming entry waits for its `confirmAt`, every other entry reissues its effect.
	/// `Fenced` moves to `Stopped`. `Failed` moves to `LoadBackoff`.
	///
	/// **Effects**
	///
	/// Dispatch runs; `Failed` arms `Load`.
	LoadStateResponse { op: OpId, result: LoadResult },
	/// **When**
	///
	/// The executor finished `WriteStateRequest`.
	///
	/// **State**
	///
	/// A response for an op other than the write in flight is stale and changes nothing. `Written`
	/// makes the carried entries `Ready` (or `Waiting` for a future confirm), removes the finished
	/// entries it deleted, and moves `persist` to `Idle`. `Fenced` moves to `Stopped`. `Refused` and
	/// `Failed` mark the entries `Dirty` again and move `persist` to `Backoff`, which holds every
	/// write until the `Persist` timer fires.
	///
	/// **Effects**
	///
	/// `RecordResult` for each deleted entry; dispatch. `Refused` and `Failed` arm `Persist` after the
	/// local backoff, or after the refusal's `retry_after_ms` when that is longer.
	WriteStateResponse { op: OpId, result: WriteResult },
	/// **When**
	///
	/// The executor finished `ListCandidatesRequest`.
	///
	/// **State**
	///
	/// A page admits selected candidates as `Copying` entries while slots and the copy byte budget
	/// last, charging each one's size to `copy_budget_unix_ms`, and advances the cursor to the last
	/// candidate handled; both are written together by the next state write. A failure moves the
	/// listing to `Backoff`.
	///
	/// **Effects**
	///
	/// `RecordResult` for skipped candidates; `RecordCopyAdmitted` for admitted ones; dispatch,
	/// which arms `Listing` while pacing or the copy budget holds the next page; a failure arms
	/// `Listing`.
	ListCandidatesResponse {
		op: OpId,
		datacenter_label: u16,
		result: ListResult,
	},
	/// **When**
	///
	/// The executor finished `CopyRequest` for a `Copying` entry.
	///
	/// **State**
	///
	/// `Copied` moves the entry to `Reserving`. `NativeActor` finishes it as `duplicate_key` for a
	/// keyed actor. Anything else moves it to `Discarding`.
	///
	/// **Effects**
	///
	/// Dispatch.
	CopyResponse {
		op: OpId,
		v2_actor_id: Id,
		result: CopyResult,
	},
	/// **When**
	///
	/// The executor finished `ReadReservationRequest`.
	///
	/// **State**
	///
	/// `Reserved` moves the entry to `CuttingOver` with a fresh `cutoverTs`. `Pending` keeps the
	/// phase and waits to read again. `OwnedElsewhere` discards it as `duplicate_key`, and
	/// `Exhausted` or `NotHeld` discard it, so a later pass imports the actor again. A failure
	/// retries.
	///
	/// **Effects**
	///
	/// Dispatch; `Pending` and a failure arm `Entry`.
	ReadReservationResponse {
		op: OpId,
		v2_actor_id: Id,
		result: ReservationResult,
	},
	/// **When**
	///
	/// The executor finished `CutoverRequest`.
	///
	/// **State**
	///
	/// `Cut` moves the entry to `Confirming` at `now + confirm_delay_ms`. A refusal moves it to
	/// `Discarding`. A failure keeps the phase and waits for a retry.
	///
	/// **Effects**
	///
	/// Dispatch; a failure arms `Entry`.
	CutoverResponse {
		op: OpId,
		v2_actor_id: Id,
		result: CutoverResult,
	},
	/// **When**
	///
	/// The executor finished `ConfirmRequest`.
	///
	/// **State**
	///
	/// `Confirmed` moves the entry to `Activating`: V3 is now the actor's only live copy. A woken
	/// or destroyed actor moves to `Discarding`. A failure keeps the phase and retries.
	///
	/// **Effects**
	///
	/// Dispatch; a failure arms `Entry`.
	ConfirmResponse {
		op: OpId,
		v2_actor_id: Id,
		result: ConfirmResult,
	},
	/// **When**
	///
	/// The executor finished `ActivateRequest`.
	///
	/// **State**
	///
	/// `Applied` finishes the entry as confirmed. `IndexPending` keeps it `Activating` and reads
	/// it again after a backoff. A failure retries. A refusal finishes it as needing recovery, as
	/// does the `HOLD_VANISHED_MAX_ATTEMPTS`th answer that the hold vanished within this phase of
	/// this tenure, whatever answers came between them (`Entry::vanished`).
	///
	/// **Effects**
	///
	/// Dispatch; `IndexPending` and a retried failure arm `Entry`.
	ActivateResponse {
		op: OpId,
		v2_actor_id: Id,
		result: ActivateResult,
	},
	/// **When**
	///
	/// The executor finished `DiscardRequest`.
	///
	/// **State**
	///
	/// Any answer but a failure finishes the entry with its discard reason. A failure retries.
	///
	/// **Effects**
	///
	/// Dispatch; a failure arms `Entry`.
	DiscardResponse {
		op: OpId,
		v2_actor_id: Id,
		result: V3Result,
	},
	/// **When**
	///
	/// The migrator service received an operator's UPS request to migrate one actor, sent by
	/// `rivet-engine migrate-from-v2 migrate`.
	///
	/// **State**
	///
	/// While loading, while another request is answered, for a datacenter without a V2 API, or
	/// with every slot taken, nothing changes. An actor already in flight becomes `requested` in
	/// `Storing`. Otherwise `requested` starts `ReadingCandidate`, which holds one slot.
	///
	/// **Effects**
	///
	/// `MigrateActorResponse` with `Unavailable` for a refusal, or `ReadCandidateRequest` with
	/// its `Io` timer; then dispatch, which replies once an in-flight entry is stored.
	MigrateActorRequest { request: RequestId, v2_actor_id: Id },
	/// **When**
	///
	/// The executor finished `ReadCandidateRequest`, or its `Io` timer fired, which counts as a
	/// failure.
	///
	/// **State**
	///
	/// A stale op changes nothing. A candidate that passes every selection rule but the share is
	/// admitted as a `Copying` entry, exactly as a listed one, and `requested` moves to `Storing`.
	/// Any other answer clears `requested`.
	///
	/// **Effects**
	///
	/// `MigrateActorResponse` with `Ineligible`, `NotFound` or `Failed` when nothing was admitted;
	/// `RecordCopyAdmitted` when the actor was admitted, which charges the copy budget but is never
	/// held back by it; dispatch, which writes the new entry and replies once it is stored.
	ReadCandidateResponse { op: OpId, result: CandidateResult },
	/// **When**
	///
	/// The executor saw a dynamic config change.
	///
	/// **State**
	///
	/// Replaces `settings`.
	///
	/// **Effects**
	///
	/// Dispatch.
	SettingsChanged { settings: Settings },
	/// **When**
	///
	/// The service is stopping because the tenure ended or the process shuts down.
	///
	/// **State**
	///
	/// Moves to `Stopped`. Durable entries stay for the next tenure.
	///
	/// **Effects**
	///
	/// None.
	StopRequest,
}
