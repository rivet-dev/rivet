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
	/// Sends `ActorMigrateFromV2Begin`, which holds the V3 actor as `MigratingFromV2` and names
	/// the incarnation whose database volume the import fills, then streams V2's export into that
	/// volume and checks its digest, then returns `CopyResponse`.
	CopyRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		candidate: wire::MigrateFromV2Candidate,
	},
	/// **When**
	///
	/// A `Reserving` entry is stored, or its wait for the reservation ended.
	///
	/// **Executor**
	///
	/// Reads the held V3 actor and reports its key reservation, then returns
	/// `ReadReservationResponse`.
	ReadReservationRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
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
	/// Sends `MigrateFromV2Activate`, which moves the held V3 actor to `Sleeping` with its alarm
	/// and index entry, reads the actor back to learn whether it applied, then returns
	/// `ActivateResponse`.
	ActivateRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
		candidate: wire::MigrateFromV2Candidate,
	},
	/// **When**
	///
	/// A `Discarding` entry is stored, or its retry wait ended.
	///
	/// **Executor**
	///
	/// Sends `MigrateFromV2Discard`, which destroys the held V3 actor and with it the imported
	/// database volume, reads the actor back to learn whether it applied, then returns
	/// `DiscardResponse`.
	DiscardRequest {
		op: OpId,
		v2_actor_id: Id,
		v3_actor_id: Id,
		migration_id: u64,
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
	/// A pass ended.
	///
	/// **Executor**
	///
	/// Sets the pass gauges from the pass counts. No response.
	ReportPassEnd { pass: wire::MigrateFromV2Pass },
}
