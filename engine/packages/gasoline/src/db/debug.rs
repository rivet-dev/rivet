use anyhow::Result;
use rivet_util::Id;

use super::Database;
use crate::history::{
	event::{RemovedEvent, SleepEvent, VersionCheckEvent},
	location::Location,
};

#[async_trait::async_trait]
pub trait DatabaseDebug: Database {
	async fn get_workflows(&self, workflow_ids: Vec<Id>) -> Result<Vec<WorkflowData>>;

	async fn find_workflows(
		&self,
		tags: &[(String, String)],
		name: Option<&str>,
		state: Option<WorkflowState>,
	) -> Result<Vec<WorkflowData>>;

	async fn silence_workflows(&self, workflow_ids: Vec<Id>) -> Result<()>;

	async fn wake_workflows(&self, workflow_ids: Vec<Id>) -> Result<()>;

	async fn get_workflow_history(
		&self,
		workflow_id: Id,
		include_forgotten: bool,
	) -> Result<Option<HistoryData>>;

	async fn get_signals(&self, signal_ids: Vec<Id>) -> Result<Vec<SignalData>>;

	async fn find_signals(
		&self,
		tags: &[(String, String)],
		workflow_id: Option<Id>,
		name: Option<&str>,
		state: Option<SignalState>,
	) -> Result<Vec<SignalData>>;

	async fn silence_signals(&self, signal_ids: Vec<Id>) -> Result<()>;

	async fn revive_workflows(
		&self,
		names: &[&str],
		error_like: &[&str],
		dry_run: bool,
	) -> Result<usize>;

	/// Lists workflows in the dead index that match the names and error substrings. Read only.
	async fn list_dead_workflows(
		&self,
		names: &[&str],
		error_like: &[&str],
	) -> Result<Vec<DeadWorkflow>>;

	async fn backfill_dead_workflows(
		&self,
		limit: usize,
		last_key: Option<&[u8]>,
	) -> Result<(usize, Option<Vec<u8>>)>;

	/// Used by pruner workflow for automatic pruning.
	async fn prune_workflows(
		&self,
		before_ts: i64,
		limit: usize,
		last_key: Option<&[u8]>,
	) -> Result<(usize, Option<Vec<u8>>)>;

	/// Used by pruner workflow for automatic pruning.
	async fn prune_signals(
		&self,
		before_ts: i64,
		limit: usize,
		last_key: Option<&[u8]>,
	) -> Result<(usize, Option<Vec<u8>>)>;

	/// Used for manual pruning.
	async fn prune_complete_workflow_history(
		&self,
		names: &[&str],
		before_ts: i64,
		dry_run: bool,
		parallelization: u16,
	) -> Result<usize>;

	/// Used for manual pruning.
	async fn prune_acked_signals(
		&self,
		names: &[&str],
		before_ts: i64,
		dry_run: bool,
		parallelization: u16,
		max_per_txn: Option<usize>,
	) -> Result<usize>;

	/// Validates that a workflow has the exact defect a repair variant handles. Read only.
	async fn inspect_workflow_repair(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Option<Location>,
	) -> Result<RepairInspection>;

	/// Applies a repair variant to a workflow. Revalidates the entire inspection inside the
	/// mutating transaction and refuses if anything changed since it was inspected.
	async fn repair_workflow(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Option<Location>,
	) -> Result<RepairOutcome>;

	/// Checks whether a repaired workflow replayed past the repair. Read only.
	async fn verify_workflow_repair(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Location,
	) -> Result<RepairVerification>;
}

/// A known workflow history defect with a targeted repair.
///
/// Each variant handles exactly one failure. They are mutually exclusive: one inserts an event, the
/// other clears one, so applying the wrong repair corrupts the workflow further. `wf repair` picks
/// the variant by validating the history rather than by trusting the persisted error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairVariant {
	/// `pegboard_actor2` died with `history diverged: expected activity "deallocate" at {...},
	/// found activity "set_error"`.
	///
	/// The workflow now runs a `set_error` activity before `deallocate` that its history predates.
	/// Inserts an incomplete `set_error` activity immediately before the existing `deallocate`, so
	/// the next wake executes it and then replays the original `deallocate` and the unchanged
	/// history tail.
	DeallocateSetError,
	/// `pegboard_actor2` died with `missing event data: event_type` at a `{loop, iteration, 1}`
	/// location.
	///
	/// Loop compaction moved the iteration's Sleep event to forgotten history and cleared its
	/// active range, then a late blind `sleep_state` write recreated a bare active row with no
	/// `event_type`, which makes the whole history undecodable. Clears the orphaned active row so
	/// active history returns to its correct post-compaction shape.
	OrphanedSleepState,
	/// Detect only. A `listen_n_until` Sleep event is followed by a Signals event in the same loop
	/// iteration, but its `sleep_state` is not `Interrupted`.
	///
	/// A Signals event can only follow that Sleep if the sleep was interrupted by those signals, so
	/// the two disagree. On replay the workflow takes the timed-out branch and diverges against the
	/// recorded Signals event. The remedy is to correct `sleep_state`, but confirming that the
	/// Signals event really belongs to this sleep requires knowing the workflow's control flow.
	SleepStateMismatch,
	/// Detect only. One loop iteration coordinate holds events in both active and forgotten
	/// history, meaning the iteration was recorded twice.
	///
	/// Compaction moves an iteration's events, so it should live in exactly one subspace.
	/// `Cursor::compare_loop_branch` reads active history only, so an iteration whose events were
	/// compacted looks new and gets a fresh branch, which a later replay then fills with a
	/// different sequence. Whether the fix is to restore the forgotten events, discard the active
	/// ones, or discard the stale iterations after them depends on which run is authoritative,
	/// which the persisted state cannot settle.
	DuplicateIterationHistory,
	/// `pegboard_actor2` replay keeps failing inside the loop's current iteration, meaning the loop
	/// event's persisted state and the events already recorded for that iteration describe
	/// different runs.
	///
	/// It surfaces either as `latent history found` (the replayed path consumed fewer events than
	/// were recorded) or as `history diverged` at a location inside the current iteration (the
	/// replayed path branched differently).
	///
	/// Advances the loop's iteration counter past every recorded branch so replay resumes on a
	/// fresh coordinate with no history to disagree with, then sends a `Lost` signal so the
	/// workflow tears down whatever it thinks it was doing instead of resuming a state the
	/// discarded branches already moved on from. `Lost` is the one signal not gated on the current
	/// transition, so it lands in every state.
	///
	/// Restricted to `pegboard_actor2`, and refused when a discarded branch references a newer
	/// actor generation than the loop state, because `Lost` only applies to the loop state's own
	/// generation and the newer one would be left running unaddressable.
	LoopIterationMismatch,
	/// Detect only. Inside one loop, an iteration branch was created before a branch at an earlier
	/// coordinate, so coordinate order and creation order disagree.
	///
	/// A single run writes iterations in ascending coordinate order, so an inversion means more
	/// than one run wrote into this loop. It says the history is untrustworthy but not which run
	/// was right.
	IterationTimestampInversion,
	/// `pegboard_actor2` died with `unreachable: ConsensusFailed { .. }`.
	///
	/// An older binary recorded a `propose` output the current code treats as unreachable, so every
	/// replay bails on that recorded output rather than on anything happening now. Clears the
	/// `propose` so the next wake runs it again against the current epoxy state.
	///
	/// Reports manual required when the `propose` is not the last recorded event, because the
	/// replay cursor is index based and clearing an earlier event shifts every event after it.
	ProposeConsensusFailed,
	/// `pegboard_actor2` died with `activity reserve_actor_key failed, max retries reached: invalid
	/// type: null, expected struct State`.
	///
	/// `insert_state_and_db` completed, so replay never runs it again, but the workflow's state key
	/// is absent, so every later activity that reads state fails. Rebuilds the state exactly as
	/// `State::new` would from that activity's recorded input, then clears the failed
	/// `reserve_actor_key` so it runs again with a fresh retry budget. Nothing is re-run, so
	/// nothing the init activity counted is counted twice.
	///
	/// Restricted to a history that is still in actor creation, because state is not recorded in
	/// history and only the init activity's value can be rebuilt exactly. A `from_v1` actor takes
	/// its `create_ts` from the pegboard actor key rather than from the activity input, so that key
	/// is read the same way the activity read it, and the repair reports manual required only when
	/// it is missing.
	MissingInitState,
}

/// Whether `wf repair` may apply a variant on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairMode {
	/// The defect has exactly one correct remedy and the inspection can fully validate it, so the
	/// repair is applied after confirmation.
	Automatic,
	/// The symptoms are recognizable but the correct remedy depends on which side of an
	/// inconsistency is authoritative, which the workflow's persisted state cannot settle. Report
	/// the symptoms and stop. A human reads the history and repairs by hand.
	ManualOnly,
}

impl RepairVariant {
	/// Every variant, in the order `wf repair` tries them.
	pub const ALL: &'static [RepairVariant] = &[
		RepairVariant::DeallocateSetError,
		RepairVariant::OrphanedSleepState,
		RepairVariant::SleepStateMismatch,
		RepairVariant::DuplicateIterationHistory,
		RepairVariant::LoopIterationMismatch,
		RepairVariant::IterationTimestampInversion,
		RepairVariant::ProposeConsensusFailed,
		RepairVariant::MissingInitState,
	];

	/// The mode a variant runs in by default. An inspection can downgrade an automatic repair to
	/// manual when the defect is recognizable but a precondition makes writing unsafe, so read
	/// `RepairInspection::mode` rather than this when reporting a specific workflow.
	pub fn mode(&self) -> RepairMode {
		match self {
			RepairVariant::DeallocateSetError
			| RepairVariant::OrphanedSleepState
			| RepairVariant::LoopIterationMismatch
			| RepairVariant::ProposeConsensusFailed
			| RepairVariant::MissingInitState => RepairMode::Automatic,
			RepairVariant::SleepStateMismatch
			| RepairVariant::DuplicateIterationHistory
			| RepairVariant::IterationTimestampInversion => RepairMode::ManualOnly,
		}
	}
}

impl std::fmt::Display for RepairVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RepairVariant::DeallocateSetError => write!(f, "deallocate-set-error"),
			RepairVariant::OrphanedSleepState => write!(f, "orphaned-sleep-state"),
			RepairVariant::SleepStateMismatch => write!(f, "sleep-state-mismatch"),
			RepairVariant::DuplicateIterationHistory => write!(f, "duplicate-iteration-history"),
			RepairVariant::LoopIterationMismatch => write!(f, "loop-iteration-mismatch"),
			RepairVariant::IterationTimestampInversion => {
				write!(f, "iteration-timestamp-inversion")
			}
			RepairVariant::ProposeConsensusFailed => write!(f, "propose-consensus-failed"),
			RepairVariant::MissingInitState => write!(f, "missing-init-state"),
		}
	}
}

#[derive(Debug)]
pub struct RepairInspection {
	pub variant: RepairVariant,
	pub workflow_id: Id,
	pub workflow_name: Option<String>,
	pub workflow_error: Option<String>,
	pub state: RepairState,
	/// Whether this workflow's repair can be applied automatically. Usually the variant's own mode,
	/// but an inspection downgrades it to `ManualOnly` when the defect is recognizable and the
	/// remedy is not safe to write without a human reading the history.
	pub mode: RepairMode,
	/// Location the repair reads or writes. Must be passed back to `verify_workflow_repair`,
	/// because after a successful repair the persisted error no longer identifies it.
	pub location: Option<Location>,
	pub has_lease: bool,
	pub has_worker: bool,
	pub has_wake_condition: bool,
	/// Every structural check run against the workflow. Printed verbatim so an operator can send
	/// back exactly what was validated.
	pub checks: Vec<RepairCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairState {
	/// Every check passed. The repair can be applied.
	Ready,
	/// The repair is already in place and the workflow has not replayed past it yet.
	AlreadyApplied,
	/// The workflow does not have this defect. The first failed check says why.
	NotApplicable,
}

#[derive(Debug)]
pub struct RepairCheck {
	pub name: String,
	pub detail: String,
	pub passed: bool,
}

impl RepairCheck {
	pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
		RepairCheck {
			name: name.into(),
			detail: detail.into(),
			passed: true,
		}
	}

	pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
		RepairCheck {
			name: name.into(),
			detail: detail.into(),
			passed: false,
		}
	}
}

#[derive(Debug)]
pub struct RepairOutcome {
	pub inspection: RepairInspection,
	/// False if the repair was already in place and this call wrote nothing.
	pub changed: bool,
	/// The repair released a stranded lease and armed an immediate wake itself, mirroring lease
	/// failover. The caller must not also wake the workflow, that would double count the workflow
	/// state metrics.
	pub wake_armed: bool,
	/// Every row the repair wrote or cleared, with the value it replaced. Printed so the mutation
	/// can be reversed by hand if the repair turns out to be wrong.
	pub mutations: Vec<String>,
}

#[derive(Debug)]
pub struct RepairVerification {
	pub variant: RepairVariant,
	pub workflow_id: Id,
	pub workflow_error: Option<String>,
	pub state: RepairVerifyState,
	pub location: Location,
	pub has_lease: bool,
	pub has_worker: bool,
	pub has_wake_condition: bool,
	pub checks: Vec<RepairCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairVerifyState {
	/// The repair is in place and the workflow replayed past it.
	Recovered,
	/// The workflow is replaying right now. Verify again once it settles.
	ReplayRunning,
	/// The repair is in place but the workflow has not been woken yet.
	AwaitingReplay,
	/// The defect came back after the workflow replayed. The underlying race fired again and the
	/// repair is not the fix.
	Regressed,
	/// The repair is in place and the workflow replayed, but it died with an unrelated error that
	/// this tool does not handle.
	UnrelatedError,
}

#[derive(Debug)]
pub struct WorkflowData {
	pub workflow_id: Id,
	pub workflow_name: String,
	pub tags: serde_json::Value,
	pub create_ts: i64,
	pub input: serde_json::Value,
	// Internally same as state, renamed to data to avoid confusion
	pub data: serde_json::Value,
	pub output: Option<serde_json::Value>,
	pub error: Option<String>,
	pub state: WorkflowState,
	/// When the workflow died. Only set while it is dead, and missing for workflows that died
	/// before this was recorded.
	pub death_ts: Option<i64>,
}

/// A workflow entry in the dead index.
#[derive(Debug)]
pub struct DeadWorkflow {
	pub workflow_id: Id,
	pub workflow_name: String,
	/// The error the workflow died with, as recorded in the dead index.
	pub error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
	Silenced,
}

#[derive(Debug)]
pub struct HistoryData {
	pub wf: WorkflowData,
	pub events: Vec<Event>,
}

#[derive(Debug)]
pub struct Event {
	pub location: Location,
	pub version: usize,
	pub create_ts: i64,
	pub forgotten: bool,
	pub data: EventData,
}

#[derive(Debug)]
pub enum EventData {
	Activity(ActivityEvent),
	Signal(SignalEvent),
	SignalSend(SignalSendEvent),
	MessageSend(MessageSendEvent),
	SubWorkflow(SubWorkflowEvent),
	Loop(LoopEvent),
	Sleep(SleepEvent),
	Removed(RemovedEvent),
	VersionCheck(VersionCheckEvent),
	Branch,
	Signals(SignalsEvent),
}

impl std::fmt::Display for EventData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self {
			EventData::Activity(activity) => write!(f, "activity {}", activity.name),
			EventData::Signal(signal) => write!(f, "signal receive {}", signal.name),
			EventData::SignalSend(signal_send) => write!(f, "signal send {}", signal_send.name),
			EventData::MessageSend(message_send) => {
				write!(f, "message send {}", message_send.name)
			}
			EventData::SubWorkflow(sub_workflow) => {
				write!(f, "sub workflow {}", sub_workflow.name)
			}
			EventData::Loop(_) => write!(f, "loop"),
			EventData::Sleep(_) => write!(f, "sleep"),
			EventData::Removed(removed) => {
				if let Some(name) = &removed.name {
					write!(f, "removed {} {name}", removed.event_type)
				} else {
					write!(f, "removed {}", removed.event_type)
				}
			}
			EventData::VersionCheck(_) => write!(f, "version check"),
			EventData::Branch => write!(f, "branch"),
			EventData::Signals(signals) => {
				let mut unique_names = signals.names.clone();
				unique_names.sort();
				unique_names.dedup();

				write!(f, "signal receive {:?}", unique_names.join(", "))
			}
		}
	}
}

#[derive(Debug)]
pub struct ActivityEvent {
	pub name: String,
	pub input: serde_json::Value,
	pub output: Option<serde_json::Value>,
	pub errors: Vec<ActivityError>,
}

#[derive(Debug)]
pub struct SignalEvent {
	pub signal_id: Id,
	pub name: String,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SignalSendEvent {
	pub signal_id: Id,
	pub name: String,
	pub workflow_id: Option<Id>,
	pub tags: Option<serde_json::Value>,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct MessageSendEvent {
	pub name: String,
	pub tags: serde_json::Value,
	pub body: serde_json::Value,
}

#[derive(Debug)]
pub struct SubWorkflowEvent {
	pub sub_workflow_id: Id,
	pub name: String,
	pub tags: serde_json::Value,
	pub input: serde_json::Value,
}

#[derive(Debug)]
pub struct LoopEvent {
	pub state: serde_json::Value,
	/// If the loop completes, this will be some.
	pub output: Option<serde_json::Value>,
	pub iteration: usize,
}

#[derive(Debug)]
pub struct SignalsEvent {
	pub signal_ids: Vec<Id>,
	pub names: Vec<String>,
	pub bodies: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ActivityError {
	pub error: String,
	pub count: usize,
	pub latest_ts: i64,
}

#[derive(Debug)]
pub struct SignalData {
	pub signal_id: Id,
	pub signal_name: String,
	pub tags: Option<serde_json::Value>,
	pub workflow_id: Option<Id>,
	pub create_ts: i64,
	pub ack_ts: Option<i64>,
	pub body: serde_json::Value,
	pub state: SignalState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalState {
	Acked,
	Pending,
	Silenced,
}
