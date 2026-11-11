//! The migrator state machine's state: one tenure's view of the durable `[V2_MIGRATOR]` entries plus
//! the I/O in flight for them.
use std::collections::BTreeMap;

use rivet_util::Id;
use serde::{Deserialize, Serialize};

use super::types::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	/// The singleton tenure (coordinator generation) that fences every write of this run.
	pub tenure: u64,
	pub settings: Settings,
	pub phase: Phase,
	/// The pass entry as this tenure last changed it. `pass_dirty` is set until a write holds it.
	pub pass: Option<wire::MigrateFromV2Pass>,
	pub pass_dirty: bool,
	pub listing: Listing,
	/// In-flight entries keyed by V2 actor id: at most `max_in_flight`, plus the operator-requested
	/// ones in `requested_entries`.
	pub entries: BTreeMap<Id, Entry>,
	pub persist: Persist,
	/// Writes that failed in a row, for the retry backoff.
	pub persist_failures: u32,
	pub next_op: u64,
	/// No page is admitted before this time, which paces admissions at `max_actors_per_sec`.
	pub next_admit_unix_ms: i64,
	/// When the copy bytes admitted so far are paid off at `max_copy_bytes_per_sec`. Each admission
	/// moves it forward by the actor's size at that rate, starting from now when it lies in the
	/// past. A listed candidate is admitted only while it is at most `COPY_BURST_MS` ahead of now.
	/// Memory only: a new tenure starts with no debt, and the `Copying` entries it loads are
	/// discarded rather than copied.
	pub copy_budget_unix_ms: i64,
	/// The one operator request to migrate an actor by id that is being answered. It lives only in
	/// memory: the request ends with this tenure, and the operator sends it again.
	pub requested: Option<Requested>,
	/// Entries an operator requested, which this tenure admitted outside `max_in_flight` (see
	/// `MAX_REQUESTED_IN_FLIGHT`). Memory only: after a failover the loaded entries are no longer
	/// told apart, which only means they count against `max_in_flight` again.
	pub requested_entries: std::collections::BTreeSet<Id>,
}

/// An operator's request to migrate one actor, whatever `migrate_ratio` says.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Requested {
	pub request: RequestId,
	pub v2_actor_id: Id,
	pub step: RequestedStep,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RequestedStep {
	/// Reading the actor's candidate facts from V2. The read holds one slot of the listing, so a
	/// page requested meanwhile leaves room for it.
	ReadingCandidate { op: OpId },
	/// The actor has an entry. The reply waits until a write stores it, so an admitted reply
	/// means the migration survives a failover.
	Storing { v3_actor_id: Id, migration_id: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
	/// The tenure is claiming the state; nothing else runs until it is loaded.
	Loading {
		op: OpId,
	},
	/// The load failed; the `Load` timer retries it.
	LoadBackoff,
	Running,
	/// A newer tenure fenced this one, or the service stopped. No further effects.
	Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Listing {
	/// Ready to request the next page once a slot is free and pacing allows.
	Idle,
	/// One candidates request is in flight for `datacenter_label`.
	Requesting { op: OpId, datacenter_label: u16 },
	/// A request failed or pacing deferred it; the `Listing` timer ends the wait.
	Backoff,
	/// Every cursor is done; the `Pass` timer starts the next pass.
	PassDone,
}

/// The state writes of this tenure. At most one write is in flight, and none while a failed
/// write's backoff runs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Persist {
	/// No write in flight. Dispatch writes changed entries and a changed pass entry.
	Idle,
	/// The one state write in flight. Entries it carries are `Step::Persisting`.
	Writing(PersistOp),
	/// The last write failed or was refused. Nothing is written until the `Persist` timer ends
	/// the backoff, so a storage host that sheds load is not sent the write again on every event.
	/// Entries changed meanwhile stay `Dirty` and ride the next write.
	Backoff,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PersistOp {
	pub op: OpId,
	/// Entries this write puts, and entries it deletes because they finished.
	pub puts: Vec<Id>,
	pub deletes: Vec<Id>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entry {
	/// The durable value. Its phase may run ahead of what is stored while `step` is `Dirty` or
	/// `Persisting`; its effect is issued only after the write lands.
	pub record: wire::MigrateFromV2InFlight,
	pub step: Step,
	/// Failed attempts of the current phase's effect, for the backoff.
	pub failures: u32,
	/// Activation answers that found no actor at the entry's placement, counted within the
	/// current phase of this tenure: any answer in between (a failure, `IndexPending`) does not
	/// reset it, a phase change does, and it is not persisted, so a takeover starts again at
	/// zero. At `HOLD_VANISHED_MAX_ATTEMPTS` the entry finishes as needing recovery.
	pub vanished: u32,
	/// Set when a copy moved the legacy-placed entry into the group, until a write stores the
	/// move; that write then tells the namespace its backfill may read again.
	pub placement_moved: bool,
	/// Set when the entry finished: the next write deletes it and then its result is recorded.
	pub finished: Option<(Outcome, Reason)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
	/// Changed since the last write; the next write carries it.
	Dirty,
	Persisting(OpId),
	/// Stored; its phase's effect may be issued.
	Ready,
	Running(OpId),
	/// Waiting for the `Entry` timer: the confirm delay or a retry backoff.
	Waiting,
}
