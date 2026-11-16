//! Outputs of the migrator state machine. Every request is idempotent by its arguments: a retry or
//! a new tenure reissues the same request, so a lost reply is harmless. The executor runs each
//! request on its own task and answers with the matching response event.
use rivet_util::Id;
use serde::{Deserialize, Serialize};

use super::types::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	/// **When**
	///
	/// The state machine starts, or the `Load` timer fires.
	///
	/// **Executor**
	///
	/// Sends `MigrateFromV2StateLoad` with the tenure, which raises the fence and returns every entry,
	/// then returns `LoadStateResponse`.
	LoadStateRequest { op: OpId, tenure: u64 },
	/// **When**
	///
	/// Dispatch finds changed entries or a changed pass entry while `persist` is `Idle`: no write is
	/// in flight and no failed write's backoff runs.
	///
	/// **Executor**
	///
	/// Sends `MigrateFromV2StateWrite` fenced by the tenure, then returns `WriteStateResponse`. A
	/// refusal that carries a Depot retry hint is returned as `Refused` with the hint.
	WriteStateRequest {
		op: OpId,
		tenure: u64,
		pass: Option<wire::MigrateFromV2Pass>,
		puts: Vec<wire::MigrateFromV2InFlight>,
		deletes: Vec<Id>,
	},
	/// **When**
	///
	/// Dispatch finds a free slot, a pending cursor, no pacing wait and copy budget left.
	///
	/// **Executor**
	///
	/// Calls V2 `candidates` on that datacenter, then returns `ListCandidatesResponse`.
	ListCandidatesRequest {
		op: OpId,
		datacenter_label: u16,
		after: Option<Vec<u8>>,
		limit: u32,
	},
	/// **When**
	///
	/// A `Copying` entry is stored.
	///
	/// **Executor**
	///
	/// Reads the standing of the entry's namespace once, or of the configured target namespace
	/// for an entry that names none. When the entry is placed at the legacy
	/// placement and the namespace has a group now, returns `CopyResponse` with `PlacementMoved`
	/// before writing anything. Otherwise sends `ActorMigrateFromV2Begin` at `group`, which holds
	/// the V3 actor as `MigratingFromV2` and names the incarnation whose database volume the
	/// import fills, then streams V2's export into that volume at `group` and checks its digest,
	/// then returns `CopyResponse`.
	CopyRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		candidate: wire::MigrateFromV2Candidate,
		/// The entry's persisted placement.
		group: Option<[u8; 16]>,
		/// The namespace the entry was admitted for.
		namespace: Option<String>,
	},
	/// **When**
	///
	/// A `Reserving` entry is stored, or its wait for the reservation ended.
	///
	/// **Executor**
	///
	/// Reads the held V3 actor at `group` and reports its key reservation, then returns
	/// `ReadReservationResponse`.
	ReadReservationRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		group: Option<[u8; 16]>,
	},
	/// **When**
	///
	/// A `CuttingOver` entry is stored, or its retry wait ended.
	///
	/// **Executor**
	///
	/// Calls V2 `cutover` with the entry's `cutoverTs`, then returns `CutoverResponse`.
	CutoverRequest {
		op: OpId,
		v2_actor_id: Id,
		generation: u32,
		cutover_ts: i64,
	},
	/// **When**
	///
	/// A `Confirming` entry's `confirmAt` passed, or its retry wait ended.
	///
	/// **Executor**
	///
	/// Calls V2 `confirm`, which parks the actor or clears the cutover, then returns
	/// `ConfirmResponse`.
	ConfirmRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		candidate: wire::MigrateFromV2Candidate,
		cutover_ts: i64,
		horizon_ms: i64,
	},
	/// **When**
	///
	/// An `Activating` entry is stored, or its retry wait ended.
	///
	/// **Executor**
	///
	/// Sends `MigrateFromV2Activate` at `group`, which moves the held V3 actor to `Sleeping` with
	/// its alarm and starts its index write, and reads the actor back to learn whether it
	/// applied; an actor missing there answers `Vanished`. At the legacy placement an activated
	/// actor answers `IndexPending` until its legacy index entry is written; once it is, the name
	/// is recorded on the volume of the namespace the actor's record names, when the namespace has
	/// one, so the backfill's walk lists the name and moves the actor into the group. Returns
	/// `ActivateResponse`.
	ActivateRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		candidate: wire::MigrateFromV2Candidate,
		group: Option<[u8; 16]>,
	},
	/// **When**
	///
	/// A `Discarding` entry is stored, or its retry wait ended.
	///
	/// **Executor**
	///
	/// Sends `MigrateFromV2Discard` at `group`, which destroys the held V3 actor and with it the
	/// imported database volume, reads the actor back to learn whether it applied, then returns
	/// `DiscardResponse`. An actor never held at `group` has nothing to discard; one that
	/// vanished after the discard was sent fails the step.
	DiscardRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		group: Option<[u8; 16]>,
	},
	/// **When**
	///
	/// An operator request to migrate an actor that is not in flight was accepted.
	///
	/// **Executor**
	///
	/// Calls V2 `candidate` on the actor's datacenter, then returns `ReadCandidateResponse`.
	ReadCandidateRequest { op: OpId, v2_actor_id: Id },
	/// **When**
	///
	/// An operator request was refused, its candidate read ended without an admission, or the
	/// entry it names was stored.
	///
	/// **Executor**
	///
	/// Hands the result to the migrator service, which replies to the operator over UPS. A
	/// request whose operator stopped waiting is dropped. No response.
	MigrateActorResponse {
		request: RequestId,
		result: MigrateActorResult,
	},
	/// **When**
	///
	/// A finished entry was deleted, or a listed candidate was skipped.
	///
	/// **Executor**
	///
	/// Increments `migration_actor_result_total`. No response.
	RecordResult { outcome: Outcome, reason: Reason },
	/// **When**
	///
	/// An entry's step failed or timed out and is retried after a backoff.
	///
	/// **Executor**
	///
	/// Increments `migrate_from_v2_effect_failures_total` for the phase. No response.
	RecordEffectFailure { phase: String },
	/// **When**
	///
	/// A listed or operator-requested actor was admitted and charged against the copy byte budget.
	///
	/// **Executor**
	///
	/// Adds `bytes` to `migrate_from_v2_copy_admitted_bytes_total`. No response.
	RecordCopyAdmitted { bytes: u64 },
	/// **When**
	///
	/// A state write landed that deleted a finished entry placed at the legacy placement, or that
	/// stored one moved into the group before its begin. Emitted once per namespace those entries
	/// name.
	///
	/// **Executor**
	///
	/// Sends `namespace`, or the configured target namespace for an entry written by version 1,
	/// a `NamespaceStepLegacyMigratorSettled` step, so a backfill that waits for the migrator's
	/// legacy-placed entries reads its state again. Best effort: a failure is logged, and the
	/// backfill's retry timer reads the state anyway. No response.
	NotifyLegacySettled { namespace: Option<String> },
	/// **When**
	///
	/// A pass ended.
	///
	/// **Executor**
	///
	/// Sets the pass gauges from the pass counts. No response.
	ReportPassEnd { pass: wire::MigrateFromV2Pass },
}
