//! The namespace state machine: one durable machine on each namespace volume, whose state is
//! the namespace's record. Two creates of one name land on the one volume, so names are unique
//! with no shared writer.
//!
//! A root's create, and the move of a namespace from its legacy catalog row, writes the namespace
//! `Creating` and creates its group record, so every namespace has a group record before it goes
//! `Live`; the outcome writes `Live` and
//! notifies the creates waiting for it. A fork writes the namespace `Forking` before the parent
//! draws a fork number, then runs the group writes with the namespace's operation id, on which every Depot step dedupes; its
//! outcome writes `Live`, or `Failed` when Depot refused it after the write, and notifies the
//! creates waiting for it. An archive, which the API admits only while no fork of the namespace
//! is live and no envoy of it is connected, writes `Archiving`, archives the group and writes
//! `Archived`. The group create, the fork and the group archive each run under the `Retry` timer, and the listing
//! index write under `IndexRetry`, so a crash anywhere is re-driven from the volume's own
//! alarm. Refusals of a request are checked by the command handler and leave nothing behind.
//!
//! Legacy: a namespace moved from its legacy row also runs its backfill, a parallel field beside
//! the phase: `Pending` until two checks one start delay apart, the first at its group create's
//! outcome, both find the fleet reading namespace volumes and a read of the V2 migrator's state
//! finds no in-flight entry at the legacy placement, then `Running`, one `MovePage` at a time under `BackfillRetry`, accepting
//! only the report of the page's current attempt, and `Done` once every actor from before groups
//! is moved into the group or given up on, every moved actor's old copy is destroyed and the
//! legacy index buckets are deleted. Removed in 3.1 with the backfill.

use anyhow::{Result, ensure};
use depot_types::keys::partition::NamespaceGroupKey;
use rivet_data::families::VersionFamily;
use rivet_data::generated::pegboard_namespace_v1 as ns;
use rivet_env::Clocks;
use rivet_keys::keys::codec::GroupId;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_durable::{DurableStateMachine, Effects, RuntimeEvent};
use serde::{Deserialize, Serialize};
use vbare::Versioned;

pub mod executor;

pub struct NamespaceStateMachine;

/// The namespace's state, or `None` before its first create. Only a create or a legacy move
/// runs on a volume without a state, and both write one, so `None` is never saved.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct State(pub Option<ns::NamespaceState>);

impl rivet_state_machine::StateMachineState for State {}

/// The machine's inputs. Commands on the namespace's volume deliver them: `NamespaceCreate`,
/// `LegacyNamespaceMove` and `NamespaceArchive` from callers, `NamespaceStep` from the executor
/// with an effect's outcome. Timers arrive through `NamespaceAlarm`.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
	/// **When**
	///
	/// The create command runs for the namespace `name`, forking `fork_from` when given.
	/// `operation_id` is the command's, so a retried command finds its own namespace.
	///
	/// **State**
	///
	/// A volume with no state is written `Creating` for a root, or `Forking` for a fork, with
	/// the waiter stored. An existing namespace changes only when the create joins its fork in
	/// progress, or is a retry of the command that is creating it, which stores the waiter;
	/// otherwise nothing changes.
	///
	/// **Effects**
	///
	/// A new namespace arms `IndexRetry` and executes `WriteIndex`; a root also arms `Retry` and
	/// executes `CreateGroup`, and a fork arms `Retry` and executes `ForkGroup`. A join or a name
	/// already taken emits nothing.
	Create {
		name: String,
		create_unix_ms: i64,
		operation_id: [u8; 16],
		fork_from: Option<ns::NamespaceParent>,
		waiter: Option<ns::NamespaceWaiter>,
	},
	/// **When**
	///
	/// Legacy: the move command runs for a namespace `name` that has only a legacy catalog row,
	/// once the fleet reads namespace volumes. The legacy behavior kept the record as that row;
	/// the new one keeps it on this volume.
	///
	/// **State**
	///
	/// A volume with no state is written with the row's create time, the index entry pending
	/// and the backfill `Pending`, and goes `Creating` in its own root group like any root. A
	/// volume with a state is left as it is, so concurrent moves converge.
	///
	/// **Effects**
	///
	/// A first move arms `IndexRetry` and executes `WriteIndex`, whose catalog write deletes the
	/// legacy row, and arms `Retry` and executes `CreateGroup`. A repeated move emits nothing.
	LegacyMove {
		name: String,
		create_unix_ms: i64,
		operation_id: [u8; 16],
	},
	/// **When**
	///
	/// The executor reports the outcome of `CreateGroup`.
	///
	/// **State**
	///
	/// Outside `Creating` it is ignored. Success writes `Live`; an unknown outcome changes
	/// nothing.
	///
	/// **Effects**
	///
	/// Success cancels `Retry` and executes `NotifyWaiters` when a waiter is current. An unknown
	/// outcome waits for `Retry`, which one carrying a refusal's hint re-arms at that hint,
	/// doubled per attempt, jittered and capped at the retry interval.
	CreateOutcome(ns::NamespaceCreateOutcome),
	/// **When**
	///
	/// The executor reports the outcome of `ForkGroup`.
	///
	/// **State**
	///
	/// Outside `Forking` it is ignored. Success writes `Live` with the group's base; a refusal
	/// writes `Failed`, retiring the name; an unknown outcome changes nothing.
	///
	/// **Effects**
	///
	/// Success or refusal cancels `Retry` and executes `NotifyWaiters` when a waiter is current.
	/// An unknown outcome waits for `Retry`, which one carrying a refusal's hint re-arms at that
	/// hint, doubled per attempt, jittered and capped at the retry interval.
	ForkOutcome(ns::NamespaceForkOutcome),
	/// **When**
	///
	/// The archive command runs after its handler refused every phase but `Live` and a
	/// namespace whose backfill is not done, and after the API refused the archive while a fork
	/// of the namespace is live or an envoy of it is connected.
	///
	/// **State**
	///
	/// A live namespace whose backfill is done goes `Archiving` at attempt 0. Any other phase,
	/// and a backfill not done, is ignored.
	///
	/// **Effects**
	///
	/// Arms `Retry` and executes `ArchiveGroup`, or nothing when ignored.
	Archive,
	/// **When**
	///
	/// The executor reports the outcome of `ArchiveGroup`.
	///
	/// **State**
	///
	/// Ignored unless the namespace is `Archiving`. Success writes `Archived`; a
	/// `group.has_children` refusal returns the namespace to `Live`; an unknown outcome changes
	/// nothing.
	///
	/// **Effects**
	///
	/// Success or refusal cancels `Retry`; an unknown outcome waits for it, and one carrying a
	/// refusal's hint re-arms it at that hint, doubled per attempt, jittered and capped at the
	/// retry interval.
	ArchiveOutcome(ns::NamespaceArchiveOutcome),
	/// **When**
	///
	/// The executor reports whether `WriteIndex` wrote the listing index entry.
	///
	/// **State**
	///
	/// A written entry while the index is pending marks it written; a failed write or a
	/// written index changes nothing.
	///
	/// **Effects**
	///
	/// A written entry cancels the pending `IndexRetry`; a failure waits for it.
	IndexOutcome(ns::NamespaceIndexOutcome),
	/// **When**
	///
	/// Legacy: the executor reports a `MovePage` that ran, with the cursor and attempt it was
	/// sent with, the actors it moved, the actor it gave up on if any, and where the walk goes
	/// next. Removed in 3.1 with the backfill.
	///
	/// **State**
	///
	/// Ignored unless the backfill is `Running` at the reported cursor and attempt, so a late
	/// report of an earlier page, or of an earlier send of this one, changes nothing. A skipped
	/// actor joins `skipped`. A next page moves the cursor, which may be a position in the walk
	/// or at its end, and resets the attempt; the end of the walk writes the backfill `Done` with
	/// the number skipped and the first `LEGACY_BACKFILL_DONE_SKIPPED_LISTED` of them.
	///
	/// **Effects**
	///
	/// Cancels the page's `BackfillRetry`. A skipped actor executes `ObserveBackfillSkipped`. A
	/// next page arms `BackfillRetry` and executes `MovePage` for it; the end of the walk
	/// executes `ObserveBackfill` for `Done` and `ObserveBackfillSkippedListed` with its list.
	BackfillOutcome(ns::NamespaceBackfillOutcome),
	/// **When**
	///
	/// Legacy: `actor_id`, an actor the backfill lists as skipped, was destroyed at its
	/// unprefixed volume, and a step command reports it: the actor API's destroy sends it, again
	/// on every retry, even once the actor's volume is gone, and the actor's own destroy effect
	/// sends it when its process's cached standing lists the actor, so a destroy through the
	/// envoy clears the entry too. Removed in 3.1 with the backfill.
	///
	/// **State**
	///
	/// Ignored unless the backfill lists the actor. A `Done` backfill drops it from `skipped` and
	/// `skipped_count` drops by one, so once none are left the namespace can be forked and
	/// archived, and its opens stop falling back. A `Running` backfill drops it from its
	/// `skipped`, which makes room below `backfill_max_skipped`, so the page waiting at the cap
	/// may give up again; its walk is already past the actor, and the end of the walk reads the
	/// destroyed actor's entry instead of passing it. A repeated report finds nothing to remove.
	///
	/// **Effects**
	///
	/// A removal executes `ObserveBackfillSkippedListed` with the list's new length.
	LegacySkippedDestroyed { actor_id: Vec<u8> },
	/// **When**
	///
	/// Legacy: the executor reports a `MovePage` that failed at its first actor without giving
	/// up on it, with the cursor, attempt and mover it was sent with and the actor's name and id.
	/// Sent for every such failure; a page that failed anywhere else reports nothing. Removed in
	/// 3.1 with the backfill.
	///
	/// **State**
	///
	/// None. The page's `BackfillRetry` sends it again.
	///
	/// **Effects**
	///
	/// When the backfill is `Running` at the reported cursor, attempt and mover, and the send
	/// had reached `backfill_max_actor_attempts` with `backfill_max_skipped` actors already
	/// skipped, so it refused to give up, executes `ObserveBackfillSkippedFull` naming the
	/// actor. Anything else, such as a failure an attempt below the last will retry, or a late
	/// report, executes nothing.
	BackfillPageFailed(ns::NamespaceBackfillPageFailed),
	/// **When**
	///
	/// Legacy: the executor reports a `LegacyReadMigratorHolds`: whether the V2 migrator holds an
	/// in-flight entry that may place an actor at this namespace's legacy placement. Removed in
	/// 3.1 with the backfill.
	///
	/// **State**
	///
	/// Ignored unless the backfill is `Pending` with `fleet_ready_seen`, the namespace is live and
	/// the fleet still reads namespace volumes. Then a report of no such entry starts the backfill
	/// `Running` with its first page; a report of one changes nothing, and the backfill waits.
	///
	/// **Effects**
	///
	/// A start cancels `BackfillRetry`, executes `ObserveBackfill` for `Running`, arms
	/// `BackfillRetry` for the first page and executes `MovePage`. A wait emits nothing: the
	/// migrator's `LegacyMigratorSettled` or the armed `BackfillRetry` reads again.
	LegacyMigratorHolds { held: bool },
	/// **When**
	///
	/// Legacy: the V2 migrator deleted in-flight entries placed at the legacy placement and
	/// notifies the target namespace, best effort; a lost notification is covered by the pending
	/// backfill's `BackfillRetry`. Removed in 3.1 with the backfill.
	///
	/// **State**
	///
	/// None.
	///
	/// **Effects**
	///
	/// While the backfill is `Pending` with `fleet_ready_seen`, the namespace is live and the fleet
	/// reads namespace volumes, executes `LegacyReadMigratorHolds`. Otherwise nothing.
	LegacyMigratorSettled,
}

impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::Create { .. } => "create",
			Event::LegacyMove { .. } => "legacy_move",
			Event::CreateOutcome(_) => "create_outcome",
			Event::ForkOutcome(_) => "fork_outcome",
			Event::Archive => "archive",
			Event::ArchiveOutcome(_) => "archive_outcome",
			Event::IndexOutcome(_) => "index_outcome",
			Event::BackfillOutcome(_) => "backfill_outcome",
			Event::LegacySkippedDestroyed { .. } => "legacy_skipped_destroyed",
			Event::BackfillPageFailed(_) => "backfill_page_failed",
			Event::LegacyMigratorHolds { .. } => "legacy_migrator_holds",
			Event::LegacyMigratorSettled => "legacy_migrator_settled",
		}
	}
}

/// The waits and limits the machine runs under, from configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
	/// The first wait for a group create, a fork's group writes, a group archive and an index
	/// write; later
	/// attempts double it up to `retry_max_ms`.
	pub fork_retry_ms: u64,
	pub retry_max_ms: u64,
	/// The most creates waiting at once for the namespace's group create or fork.
	pub start_waiters_capacity: usize,
	/// Legacy: the wait before a pending backfill checks the fleet, after the move and between
	/// checks. Removed in 3.1 with the backfill.
	pub backfill_start_delay_ms: u64,
	/// Legacy: how long one backfill page runs before `BackfillRetry` sends it again: the
	/// configured page timeout, raised to cover an actor's stop as
	/// `legacy_backfill::page_timeout_ms` does. Removed in 3.1 with the backfill.
	pub backfill_page_timeout_ms: u64,
	/// Legacy: the sends of one page after which a page gives up on an actor it cannot move.
	/// Removed in 3.1 with the backfill.
	pub backfill_max_actor_attempts: u32,
	/// Legacy: the actors a running backfill may give up on; at it, a page that reaches its
	/// attempts fails instead. Removed in 3.1 with the backfill.
	pub backfill_max_skipped: usize,
}

impl Default for Policy {
	fn default() -> Self {
		Self {
			fork_retry_ms: 15_000,
			retry_max_ms: 30_000,
			start_waiters_capacity: 64,
			backfill_start_delay_ms: 600_000,
			backfill_page_timeout_ms: 120_000,
			backfill_max_actor_attempts: 10,
			backfill_max_skipped: LEGACY_BACKFILL_DONE_SKIPPED_LISTED,
		}
	}
}

/// The configuration a transition applies. Nothing is read: a namespace's transitions decide
/// on its own state and the event alone.
#[derive(Clone, Debug, Default)]
pub struct Preparation {
	pub policy: Policy,
	/// The region's event shards, which a group archive's event is posted to.
	pub event_shards: u16,
	/// The index bucket count a new root lineage is fixed at.
	pub index_bucket_count: u64,
	/// Legacy: whether every storage worker reads namespace volumes, so no binary from before
	/// groups is known to run and a pending backfill may start. Read from the in-memory
	/// topology view before the transition. Removed in 3.1 with the backfill.
	pub backfill_fleet_ready: bool,
}

/// The machine's outgoing work. Every effect's executor runs in a spawned task after the
/// transition commits and reports back through a `NamespaceStep` command on this volume; a lost
/// report costs one timer firing. `attempt`, where carried, is 0 for the first send and counts
/// re-sends.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
	/// **When**
	///
	/// A root's create, or a legacy move, writes `Creating`, and every `Retry` while
	/// it stays `Creating`.
	///
	/// **Executor**
	///
	/// Creates the root group record of `group`, keyed by the namespace's group key; an
	/// existing root record answers as created, so a re-send is idempotent. Returns
	/// `CreateOutcome`.
	CreateGroup { group: GroupId, attempt: u32 },
	/// **When**
	///
	/// A fork's create writes `Forking`, and every `Retry` while it stays `Forking`.
	///
	/// **Executor**
	///
	/// Forks the group `child` from `parent` with `operation_id`, on which every Depot step
	/// dedupes. The parent's record exists, since every namespace with a group created it
	/// before going `Live`. Returns `ForkOutcome`.
	ForkGroup {
		parent: GroupId,
		child: GroupId,
		operation_id: [u8; 16],
		unix_ms: i64,
		attempt: u32,
	},
	/// **When**
	///
	/// An archive of a namespace starts, and every `Retry` while it is `Archiving`.
	///
	/// **Executor**
	///
	/// Archives the group and posts the archive to the region's event shards. Returns
	/// `ArchiveOutcome`.
	ArchiveGroup {
		group: GroupId,
		shards: u16,
		attempt: u32,
	},
	/// **When**
	///
	/// A create or a legacy move writes a new namespace, and every `IndexRetry` while the index
	/// is pending.
	///
	/// **Executor**
	///
	/// Writes the namespace's listing index entry on the catalog, deleting its legacy row in
	/// the same transaction. Returns `IndexOutcome`.
	WriteIndex { attempt: u32, create_unix_ms: i64 },
	/// **When**
	///
	/// A namespace leaves `Creating` or `Forking` while some waiter has not expired.
	///
	/// **Executor**
	///
	/// Sends the namespace's state to each waiting create's subject; a lost notice is covered
	/// by the create reading the namespace at its deadline. Returns nothing.
	NotifyWaiters {
		waiters: Vec<ns::NamespaceWaiter>,
		state: ns::NamespaceState,
	},
	/// **When**
	///
	/// Legacy: a pending backfill starts, a page reports a next page, and every
	/// `BackfillRetry` while the backfill is `Running`. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Runs in a task the namespace volume's opening owns, so losing the volume cancels it, and
	/// aborts this opening's previous page of the namespace. Every write the page makes to an
	/// actor names `mover`, which the actor refuses once a later page's mover marked it. It walks
	/// the namespace's legacy
	/// index buckets from `cursor`, passing the actors in `skipped`, and moves each actor it
	/// finds into `group`, one after another, up to a page or half the page timeout, the whole
	/// page bounded by the page timeout. At the end of the walk it probes every name's buckets,
	/// restarting the walk where it finds an actor left behind, waits for the moved actors' old
	/// copies to be destroyed, and deletes the legacy buckets, under the same deadline: a page
	/// that runs out of time there reports the bucket it reached as the next position, where the
	/// next page resumes. Returns `BackfillOutcome` with the
	/// cursor and attempt it was sent with. When the page fails at its first actor without
	/// `give_up` it returns `BackfillPageFailed` naming that actor, and `BackfillRetry`
	/// re-drives the page; with `give_up` set it gives up on that actor and reports it skipped
	/// with the next page past it. Sets `pegboard_backfill_skipped_listed` to the length of
	/// `skipped`, so a process that took the namespace's volume over reports the running list
	/// from its first page.
	MovePage {
		group: GroupId,
		index_bucket_count: u64,
		cursor: Option<ns::NamespaceBackfillPosition>,
		attempt: u32,
		skipped: Vec<ns::NamespaceBackfillSkipped>,
		give_up: bool,
		/// The page's mover token, `NamespaceBackfillRunning.mover`.
		mover: u64,
	},
	/// **When**
	///
	/// Legacy: the backfill enters a phase: `Pending` at a legacy move, `Running` at its start,
	/// `Done` at its end. Executed after the transition commits, so a failed commit counts
	/// nothing. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Counts the entry in `pegboard_backfill_phase_entries_total`. Returns nothing.
	ObserveBackfill { phase: BackfillPhase },
	/// **When**
	///
	/// Legacy: an accepted `BackfillOutcome` reports an actor the page gave up on. Executed after
	/// the transition commits. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Logs an error naming the namespace, the actor's name and its id, for an operator to
	/// resolve before 3.1, and counts it in `pegboard_backfill_skipped_total`. Sets
	/// `pegboard_backfill_skipped_listed` to `listed`, the actors the backfill now carries, and
	/// its capacity gauge to `capacity`, and warns from 90% of it. Returns nothing.
	ObserveBackfillSkipped {
		skipped: ns::NamespaceBackfillSkipped,
		listed: u32,
		capacity: u32,
	},
	/// **When**
	///
	/// Legacy: an accepted `BackfillPageFailed` reports the page that reached
	/// `backfill_max_actor_attempts` while the backfill carried `backfill_max_skipped` skipped
	/// actors, so it was sent without a give-up and failed at `actor_id`, listed under
	/// `actor_name`. Executed after the transition commits. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Logs an error naming the namespace and the actor, telling the operator the backfill waits
	/// on it until it is destroyed or repaired, or a skipped actor is destroyed to make room;
	/// counts `pegboard_backfill_give_ups_refused_total` and sets the skipped list gauges.
	/// Returns nothing.
	ObserveBackfillSkippedFull {
		actor_name: String,
		actor_id: Vec<u8>,
		listed: u32,
		capacity: u32,
	},
	/// **When**
	///
	/// Legacy: the backfill's list of skipped actors changed without a skip: the backfill
	/// reached `Done`, carrying its `Done` list, or a destroyed skipped actor left the list.
	/// Executed after the transition commits. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Sets `pegboard_backfill_skipped_listed` to `listed`, the actors the list now holds.
	/// Returns nothing.
	ObserveBackfillSkippedListed { listed: u32 },
	/// **When**
	///
	/// Legacy: a pending backfill found the fleet ready on two checks one start delay apart, at
	/// that `BackfillRetry` and every later one while it stays pending, and on each
	/// `LegacyMigratorSettled`. Removed in 3.1 with the backfill.
	///
	/// **Executor**
	///
	/// Reports `LegacyMigratorHolds` with whether the V2 migrator holds an in-flight entry at the
	/// legacy placement for this namespace: none while `migrate_from_v2` targets another
	/// namespace or is not configured, else one read of the migrator's state, without claiming
	/// it, for an entry with no volume group. A failed read reports nothing, and `BackfillRetry`
	/// reads again.
	LegacyReadMigratorHolds,
}

/// Legacy: a backfill phase, as `ObserveBackfill` counts it. Removed in 3.1 with the backfill.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackfillPhase {
	Pending,
	Running,
	Done,
}

impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::CreateGroup { .. } => "create_group",
			Effect::ForkGroup { .. } => "fork_group",
			Effect::ArchiveGroup { .. } => "archive_group",
			Effect::WriteIndex { .. } => "write_index",
			Effect::NotifyWaiters { .. } => "notify_waiters",
			Effect::MovePage { .. } => "move_page",
			Effect::ObserveBackfill { .. } => "observe_backfill",
			Effect::ObserveBackfillSkipped { .. } => "observe_backfill_skipped",
			Effect::ObserveBackfillSkippedFull { .. } => "observe_backfill_skipped_full",
			Effect::ObserveBackfillSkippedListed { .. } => "observe_backfill_skipped_listed",
			Effect::LegacyReadMigratorHolds => "legacy_read_migrator_holds",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot;

impl rivet_state_machine::Snapshot for Snapshot {}

type NamespaceEffects = Effects<Effect, ns::NamespaceTimer, ()>;

impl DurableStateMachine for NamespaceStateMachine {
	type Slot = NoSlot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Timer = ns::NamespaceTimer;
	type Snapshot = Snapshot;
	type Preparation = Preparation;
	type WriteEffect = ();

	const DEBUG_NAME: &'static str = "namespace";
	const FAMILY: VersionFamily = VersionFamily::PEGBOARD_NAMESPACE;

	fn start(
		_state: &mut Self::State,
		_clocks: Clocks,
		_entropy: Entropy,
		_effects: &mut NamespaceEffects,
	) {
	}

	fn on_event(
		state: &mut Self::State,
		raw_event: rivet_state_machine::Event<RuntimeEvent<Event, ns::NamespaceTimer>>,
		preparation: Preparation,
		effects: &mut NamespaceEffects,
	) {
		let now_unix_ms = raw_event.now.unix_ms;
		let entropy = raw_event.entropy;
		match raw_event.body {
			RuntimeEvent::Input(Event::Create {
				name,
				create_unix_ms,
				operation_id,
				fork_from,
				waiter,
			}) => create(
				state,
				&name,
				create_unix_ms,
				operation_id,
				fork_from,
				waiter,
				&preparation,
				now_unix_ms,
				effects,
			),
			RuntimeEvent::Input(Event::LegacyMove {
				name,
				create_unix_ms,
				operation_id,
			}) => legacy_move(
				state,
				&name,
				create_unix_ms,
				operation_id,
				&preparation,
				effects,
			),
			RuntimeEvent::Input(Event::CreateOutcome(outcome)) => {
				create_outcome(state, outcome, &preparation, entropy, now_unix_ms, effects)
			}
			RuntimeEvent::Input(Event::ForkOutcome(outcome)) => {
				fork_outcome(state, outcome, &preparation, entropy, now_unix_ms, effects)
			}
			RuntimeEvent::Input(Event::Archive) => archive(state, &preparation, effects),
			RuntimeEvent::Input(Event::ArchiveOutcome(outcome)) => {
				archive_outcome(state, outcome, &preparation, entropy, now_unix_ms, effects)
			}
			RuntimeEvent::Input(Event::IndexOutcome(outcome)) => {
				index_outcome(state, outcome, effects)
			}
			RuntimeEvent::Input(Event::BackfillOutcome(outcome)) => {
				legacy_backfill_outcome(state, outcome, &preparation, effects)
			}
			RuntimeEvent::Input(Event::LegacySkippedDestroyed { actor_id }) => {
				legacy_skipped_destroyed(state, &actor_id, effects)
			}
			RuntimeEvent::Input(Event::BackfillPageFailed(failed)) => {
				legacy_backfill_page_failed(state, failed, &preparation.policy, effects)
			}
			RuntimeEvent::Input(Event::LegacyMigratorHolds { held }) => {
				legacy_migrator_holds(state, held, &preparation, effects)
			}
			RuntimeEvent::Input(Event::LegacyMigratorSettled) => {
				if legacy_awaits_migrator(state, &preparation) {
					effects.execute(Effect::LegacyReadMigratorHolds);
				}
			}
			RuntimeEvent::Timer(ns::NamespaceTimer::NamespaceRetry) => {
				retry(state, &preparation, entropy, now_unix_ms, effects)
			}
			RuntimeEvent::Timer(ns::NamespaceTimer::NamespaceIndexRetry(timer)) => {
				index_retry(state, timer.attempt, &preparation, entropy, effects)
			}
			RuntimeEvent::Timer(ns::NamespaceTimer::NamespaceBackfillRetry(timer)) => {
				legacy_backfill_retry(state, timer.attempt, &preparation, effects)
			}
		}
	}

	fn snapshot(_state: &Self::State) -> Self::Snapshot {
		Snapshot
	}

	/// A namespace's volume is in no forkable group, so it is never made by a group fork.
	fn fork_reset(_state: &Self::State) -> Option<[u8; 16]> {
		None
	}

	/// Waiters exist only while a group create or fork is outstanding, and a creating, forking
	/// or archiving namespace has a group.
	fn debug_validate(state: &Self::State) -> Result<()> {
		let Some(namespace) = &state.0 else {
			return Ok(());
		};
		let forking = matches!(namespace.phase, ns::NamespacePhase::NamespaceForking(_));
		let creating = matches!(namespace.phase, ns::NamespacePhase::NamespaceCreating(_));
		let archiving = matches!(namespace.phase, ns::NamespacePhase::NamespaceArchiving(_));
		ensure!(
			namespace.group.is_some(),
			"a namespace on its own volume has a group"
		);
		ensure!(
			!archiving
				|| matches!(
					&namespace.backfill,
					ns::NamespaceBackfill::NamespaceBackfillDone(done) if done.skipped_count == 0
				),
			"an archiving namespace has finished its backfill with nothing skipped"
		);
		ensure!(
			namespace.start_waiters.is_empty() || forking || creating,
			"a namespace holds start waiters only while it is creating or forking"
		);
		Ok(())
	}

	fn serialize_state(state: &Self::State, version: u16) -> Result<Vec<u8>> {
		let Some(namespace) = &state.0 else {
			anyhow::bail!("a namespace state is saved only once a create wrote it");
		};
		rivet_data::versioned::NamespaceState::serialize_with_embedded_version(namespace, version)
	}

	fn deserialize_state(bytes: &[u8]) -> Result<Self::State> {
		rivet_data::versioned::NamespaceState::deserialize_with_embedded_version(bytes)
			.map(|namespace| State(Some(namespace)))
	}

	/// The machine runs one retry, one index retry and one backfill retry at a time, and each
	/// handler ignores a stale attempt, so the variant alone is the id. Stored timers keep their
	/// ids.
	fn timer_id(timer: &Self::Timer) -> Vec<u8> {
		let id: u32 = match timer {
			ns::NamespaceTimer::NamespaceRetry => 1,
			ns::NamespaceTimer::NamespaceIndexRetry(_) => 2,
			ns::NamespaceTimer::NamespaceBackfillRetry(_) => 3,
		};
		id.to_be_bytes().to_vec()
	}

	fn serialize_timer(timer: &Self::Timer, version: u16) -> Result<Vec<u8>> {
		rivet_data::versioned::NamespaceTimer::serialize_with_embedded_version(timer, version)
	}

	fn deserialize_timer(bytes: &[u8]) -> Result<Self::Timer> {
		rivet_data::versioned::NamespaceTimer::deserialize_with_embedded_version(bytes)
	}
}

/// The wait before attempt `attempt` of an effect: the first wait doubled per attempt up to
/// the cap, plus up to a quarter of jitter so the namespaces of one crash do not re-drive in
/// lockstep.
fn backoff_ms(policy: &Policy, attempt: u32, entropy: Entropy) -> u64 {
	let multiplier = 1u64.checked_shl(attempt.min(16)).unwrap_or(u64::MAX);
	let delay = policy
		.fork_retry_ms
		.saturating_mul(multiplier)
		.min(policy.retry_max_ms);
	let jitter = entropy.rng().gen_range(0..delay / 4 + 1);
	delay.saturating_add(jitter)
}

/// The group a new root namespace gets: its own, derived from its name, so it is known before
/// any write.
fn root_group(name: &str, index_bucket_count: u64) -> ns::NamespaceGroup {
	let group = NamespaceGroupKey::new(name.to_owned()).group_id();
	ns::NamespaceGroup {
		group,
		root: group,
		depth: 0,
		base: None,
		index_bucket_count,
	}
}

/// Stores a create's waiter while the group create or fork is outstanding. Expired waiters are pruned first; a
/// waiter already present keeps its slot and takes the later expiry. A full list leaves the
/// waiter out, and that create reads the namespace at its deadline.
fn store_waiter(
	namespace: &mut ns::NamespaceState,
	waiter: Option<ns::NamespaceWaiter>,
	policy: &Policy,
	now_unix_ms: i64,
) {
	let Some(waiter) = waiter else {
		return;
	};
	namespace
		.start_waiters
		.retain(|stored| stored.expires_unix_ms > now_unix_ms);
	let existing = namespace
		.start_waiters
		.iter_mut()
		.find(|stored| stored.worker_id == waiter.worker_id && stored.wait_id == waiter.wait_id);
	if let Some(existing) = existing {
		existing.expires_unix_ms = existing.expires_unix_ms.max(waiter.expires_unix_ms);
		return;
	}
	if namespace.start_waiters.len() >= policy.start_waiters_capacity {
		return;
	}
	namespace.start_waiters.push(waiter);
}

/// Sends the namespace's state to every waiter that has not expired and takes them.
fn notify_waiters(
	namespace: &mut ns::NamespaceState,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	let waiters = std::mem::take(&mut namespace.start_waiters)
		.into_iter()
		.filter(|waiter| waiter.expires_unix_ms > now_unix_ms)
		.collect::<Vec<_>>();
	if waiters.is_empty() {
		return;
	}
	effects.execute(Effect::NotifyWaiters {
		waiters,
		state: namespace.clone(),
	});
}

/// Writes a root namespace `Creating` in its own group and starts its group create, which takes
/// it `Live`. Every root and every legacy move goes through here, so a namespace is never `Live`
/// before its group record exists. `backfill` is `Done` for a new root and `Pending` for a
/// namespace moved from its legacy row.
fn creating_root(
	name: &str,
	create_unix_ms: i64,
	operation_id: [u8; 16],
	backfill: ns::NamespaceBackfill,
	preparation: &Preparation,
	effects: &mut NamespaceEffects,
) -> ns::NamespaceState {
	let group = root_group(name, preparation.index_bucket_count);
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceRetry,
		preparation.policy.fork_retry_ms,
	);
	effects.execute(Effect::CreateGroup {
		group: group.group,
		attempt: 0,
	});
	ns::NamespaceState {
		create_unix_ms,
		operation_id,
		group: Some(group),
		phase: ns::NamespacePhase::NamespaceCreating(ns::NamespaceCreating { attempt: 0 }),
		index: ns::NamespaceIndexState::NamespaceIndexPending(ns::NamespaceIndexPending {
			attempt: 0,
		}),
		backfill,
		start_waiters: Vec::new(),
	}
}

/// Starts the listing index write of a namespace this transition created.
fn start_index_write(
	namespace: &ns::NamespaceState,
	policy: &Policy,
	effects: &mut NamespaceEffects,
) {
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceIndexRetry(ns::NamespaceIndexRetry { attempt: 0 }),
		policy.fork_retry_ms,
	);
	effects.execute(Effect::WriteIndex {
		attempt: 0,
		create_unix_ms: namespace.create_unix_ms,
	});
}

/// Creates the namespace, or leaves an existing one as it is. A create forking the same parent
/// as a fork in progress joins it as one more waiter, as does a retry of the command whose root
/// is still `Creating`; the handler tells the caller whether the namespace is its own from the
/// operation id and the phase.
#[allow(clippy::too_many_arguments, reason = "one transition's inputs")]
fn create(
	state: &mut State,
	name: &str,
	create_unix_ms: i64,
	operation_id: [u8; 16],
	fork_from: Option<ns::NamespaceParent>,
	waiter: Option<ns::NamespaceWaiter>,
	preparation: &Preparation,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	if let Some(namespace) = &mut state.0 {
		let retries_own_create = namespace.operation_id == operation_id
			&& matches!(namespace.phase, ns::NamespacePhase::NamespaceCreating(_));
		if retries_own_create || joins_fork(namespace, fork_from.as_ref()) {
			store_waiter(namespace, waiter, &preparation.policy, now_unix_ms);
		}
		return;
	}

	let namespace = match fork_from {
		None => {
			let mut namespace = creating_root(
				name,
				create_unix_ms,
				operation_id,
				ns::NamespaceBackfill::NamespaceBackfillDone(ns::NamespaceBackfillDone {
					skipped: Vec::new(),
					skipped_count: 0,
				}),
				preparation,
				effects,
			);
			store_waiter(&mut namespace, waiter, &preparation.policy, now_unix_ms);
			namespace
		}
		Some(parent) => {
			let child = NamespaceGroupKey::new(name.to_owned()).group_id();
			let mut namespace = ns::NamespaceState {
				create_unix_ms,
				operation_id,
				group: Some(ns::NamespaceGroup {
					group: child,
					root: parent.group.root,
					depth: parent.group.depth.saturating_add(1),
					base: None,
					index_bucket_count: parent.group.index_bucket_count,
				}),
				phase: ns::NamespacePhase::NamespaceForking(ns::NamespaceForking {
					parent_namespace: parent.name.clone(),
					parent: parent.group.group,
					attempt: 0,
				}),
				index: ns::NamespaceIndexState::NamespaceIndexPending(ns::NamespaceIndexPending {
					attempt: 0,
				}),
				// Only a namespace whose backfill is done can be forked, so a fork has nothing
				// unprefixed to copy.
				backfill: ns::NamespaceBackfill::NamespaceBackfillDone(ns::NamespaceBackfillDone {
					skipped: Vec::new(),
					skipped_count: 0,
				}),
				start_waiters: Vec::new(),
			};
			store_waiter(&mut namespace, waiter, &preparation.policy, now_unix_ms);
			effects.arm_timer_after(
				ns::NamespaceTimer::NamespaceRetry,
				preparation.policy.fork_retry_ms,
			);
			effects.execute(Effect::ForkGroup {
				parent: parent.group.group,
				child,
				operation_id,
				unix_ms: now_unix_ms,
				attempt: 0,
			});
			namespace
		}
	};
	start_index_write(&namespace, &preparation.policy, effects);
	state.0 = Some(namespace);
}

/// Whether a create forking `fork_from` is the fork of `namespace` already in progress.
pub fn joins_fork(namespace: &ns::NamespaceState, fork_from: Option<&ns::NamespaceParent>) -> bool {
	let Some(parent) = fork_from else {
		return false;
	};
	match &namespace.phase {
		ns::NamespacePhase::NamespaceForking(forking) => forking.parent_namespace == parent.name,
		ns::NamespacePhase::NamespaceCreating(_)
		| ns::NamespacePhase::NamespaceLive
		| ns::NamespacePhase::NamespaceArchiving(_)
		| ns::NamespacePhase::NamespaceArchived(_)
		| ns::NamespacePhase::NamespaceFailed(_) => false,
	}
}

/// Whether the namespace holds `waiter`, so a create can tell a stored waiter from one the full
/// list left out.
pub fn has_waiter(namespace: &ns::NamespaceState, waiter: &ns::NamespaceWaiter) -> bool {
	namespace
		.start_waiters
		.iter()
		.any(|stored| stored.worker_id == waiter.worker_id && stored.wait_id == waiter.wait_id)
}

/// Legacy: writes a namespace moved from its legacy catalog row. The legacy behavior kept the
/// record as that row; the new one keeps it on the namespace's volume. Every legacy row is a
/// live namespace, which goes `Creating` in its own root group like any root, so its group
/// record exists before it is `Live` again, with its backfill `Pending`: its volumes from before
/// groups are still unprefixed. The machine writes the index entry under `IndexRetry`, and that
/// write deletes the row, so a crash after this transition still finishes the move.
fn legacy_move(
	state: &mut State,
	name: &str,
	create_unix_ms: i64,
	operation_id: [u8; 16],
	preparation: &Preparation,
	effects: &mut NamespaceEffects,
) {
	if state.0.is_some() {
		return;
	}
	let namespace = creating_root(
		name,
		create_unix_ms,
		operation_id,
		ns::NamespaceBackfill::NamespaceBackfillPending(ns::NamespaceBackfillPending {
			fleet_ready_seen: false,
		}),
		preparation,
		effects,
	);
	start_index_write(&namespace, &preparation.policy, effects);
	effects.execute(Effect::ObserveBackfill {
		phase: BackfillPhase::Pending,
	});
	state.0 = Some(namespace);
}

/// Takes a root `Live` once its group record exists and notifies its waiters; an unknown
/// outcome waits for `Retry`, which creates the record again. A root with a pending backfill
/// arms its start.
fn create_outcome(
	state: &mut State,
	outcome: ns::NamespaceCreateOutcome,
	preparation: &Preparation,
	entropy: Entropy,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let ns::NamespacePhase::NamespaceCreating(creating) = &namespace.phase else {
		return;
	};
	let attempt = creating.attempt;
	match outcome {
		ns::NamespaceCreateOutcome::NamespaceCreateOutcomeOk => {
			effects.cancel_timer(ns::NamespaceTimer::NamespaceRetry);
			namespace.phase = ns::NamespacePhase::NamespaceLive;
			notify_waiters(namespace, now_unix_ms, effects);
			// Legacy: a namespace moved from its legacy row waits one start delay before its
			// backfill checks the fleet again, so stateless workers rolling after the storage
			// workers finish first. Its move is the first check: a namespace moves only once the
			// fleet reads namespace volumes, and this outcome records whether it still does in
			// the timer it arms, not in `fleet_ready_seen`, so no report or notification before
			// the timer can start the backfill. Removed in 3.1 with the backfill.
			if matches!(
				namespace.backfill,
				ns::NamespaceBackfill::NamespaceBackfillPending(_)
			) {
				let attempt = if preparation.backfill_fleet_ready {
					LEGACY_MOVE_FOUND_FLEET_READY
				} else {
					0
				};
				effects.arm_timer_after(
					ns::NamespaceTimer::NamespaceBackfillRetry(ns::NamespaceBackfillRetry {
						attempt,
					}),
					preparation.policy.backfill_start_delay_ms,
				);
			}
		}
		ns::NamespaceCreateOutcome::NamespaceOutcomeUnknown(unknown) => {
			retry_refused_promptly(&unknown, attempt, &preparation.policy, entropy, effects)
		}
	}
}

/// An effect's group write was refused before it ran, with `unknown`'s hint: the retry timer is
/// moved up to the hint, doubled for each earlier attempt and jittered so namespaces refused
/// together do not retry together, never later than the configured retry interval. Without a
/// hint, nothing is known of the write and the armed retry interval stands.
fn retry_refused_promptly(
	unknown: &ns::NamespaceOutcomeUnknown,
	attempt: u32,
	policy: &Policy,
	entropy: Entropy,
	effects: &mut NamespaceEffects,
) {
	let Some(retry_after_ms) = unknown.retry_after_ms else {
		return;
	};
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceRetry,
		refused_retry_ms(policy, attempt, retry_after_ms, entropy),
	);
}

/// The wait before re-driving an effect refused with hint `retry_after_ms` at `attempt`: the
/// hint doubled per earlier attempt, capped at the configured retry interval, then between half
/// and all of that.
fn refused_retry_ms(policy: &Policy, attempt: u32, retry_after_ms: u64, entropy: Entropy) -> u64 {
	let multiplier = 1u64.checked_shl(attempt.min(16)).unwrap_or(u64::MAX);
	let delay = retry_after_ms
		.max(1)
		.saturating_mul(multiplier)
		.min(policy.fork_retry_ms);
	let half = delay / 2;
	half.saturating_add(entropy.rng().gen_range(0..delay - half + 1))
}

fn fork_outcome(
	state: &mut State,
	outcome: ns::NamespaceForkOutcome,
	preparation: &Preparation,
	entropy: Entropy,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let ns::NamespacePhase::NamespaceForking(forking) = &namespace.phase else {
		return;
	};
	let parent_namespace = forking.parent_namespace.clone();
	let attempt = forking.attempt;
	match outcome {
		ns::NamespaceForkOutcome::NamespaceForkOutcomeOk(ok) => {
			effects.cancel_timer(ns::NamespaceTimer::NamespaceRetry);
			if let Some(group) = &mut namespace.group {
				group.base = Some(ns::NamespaceBase {
					parent_namespace,
					fork_number: ok.fork_number,
					forked_unix_ms: now_unix_ms,
				});
			}
			namespace.phase = ns::NamespacePhase::NamespaceLive;
			notify_waiters(namespace, now_unix_ms, effects);
		}
		ns::NamespaceForkOutcome::NamespaceOutcomeRefused(refused) => {
			// Nothing on the group volumes needs undoing: an unused fork number on the parent
			// is harmless. The name stays, retired, since names are never reused.
			effects.cancel_timer(ns::NamespaceTimer::NamespaceRetry);
			namespace.phase = ns::NamespacePhase::NamespaceFailed(ns::NamespaceFailed {
				reason: refused.code,
			});
			notify_waiters(namespace, now_unix_ms, effects);
		}
		ns::NamespaceForkOutcome::NamespaceOutcomeUnknown(unknown) => {
			retry_refused_promptly(&unknown, attempt, &preparation.policy, entropy, effects)
		}
	}
}

/// Starts an archive of a live namespace: it goes `Archiving`, so every process refuses new
/// work for it within the status TTL, and its group is archived, which closes its volumes at
/// every tablet. The API refused the archive while an envoy of the namespace was connected; an
/// envoy that connects in the status TTL window is not stopped, and its next open answers
/// `group.archived`. The handler refused every other phase before the transition.
fn archive(state: &mut State, preparation: &Preparation, effects: &mut NamespaceEffects) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	if !matches!(namespace.phase, ns::NamespacePhase::NamespaceLive) {
		return;
	}
	// The handler refused a namespace whose backfill is not done or skipped actors: its
	// unprefixed volumes are in no group, so archiving the group would not close them.
	let settled = matches!(
		&namespace.backfill,
		ns::NamespaceBackfill::NamespaceBackfillDone(done) if done.skipped_count == 0
	);
	if !settled {
		return;
	}
	let Some(group) = namespace.group.as_ref().map(|group| group.group) else {
		return;
	};
	namespace.phase = ns::NamespacePhase::NamespaceArchiving(ns::NamespaceArchiving { attempt: 0 });
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceRetry,
		preparation.policy.fork_retry_ms,
	);
	effects.execute(Effect::ArchiveGroup {
		group,
		shards: preparation.event_shards,
		attempt: 0,
	});
}

fn archive_outcome(
	state: &mut State,
	outcome: ns::NamespaceArchiveOutcome,
	preparation: &Preparation,
	entropy: Entropy,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let ns::NamespacePhase::NamespaceArchiving(archiving) = &namespace.phase else {
		return;
	};
	let attempt = archiving.attempt;
	match outcome {
		ns::NamespaceArchiveOutcome::NamespaceArchiveOutcomeOk => {
			effects.cancel_timer(ns::NamespaceTimer::NamespaceRetry);
			namespace.phase = ns::NamespacePhase::NamespaceArchived(ns::NamespaceArchived {
				archived_unix_ms: now_unix_ms,
			});
		}
		ns::NamespaceArchiveOutcome::NamespaceOutcomeRefused(refused) => {
			// The executor reports only `group.has_children` as a refusal: a fork of the
			// namespace was made between the API's check and the archive. The namespace is live
			// again, and the caller archives the fork and calls again.
			tracing::warn!(
				code = %refused.code,
				"the group archive of a namespace was refused because it has children that are not archived; the namespace stays live"
			);
			effects.cancel_timer(ns::NamespaceTimer::NamespaceRetry);
			namespace.phase = ns::NamespacePhase::NamespaceLive;
		}
		ns::NamespaceArchiveOutcome::NamespaceOutcomeUnknown(unknown) => {
			retry_refused_promptly(&unknown, attempt, &preparation.policy, entropy, effects)
		}
	}
}

/// Marks the index entry written. Any written outcome counts, whichever attempt wrote it, since
/// every attempt writes the same entry; a failure waits for `IndexRetry`.
fn index_outcome(
	state: &mut State,
	outcome: ns::NamespaceIndexOutcome,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let ns::NamespaceIndexState::NamespaceIndexPending(pending) = &namespace.index else {
		return;
	};
	if !outcome.written {
		return;
	}
	effects.cancel_timer(ns::NamespaceTimer::NamespaceIndexRetry(
		ns::NamespaceIndexRetry {
			attempt: pending.attempt,
		},
	));
	namespace.index = ns::NamespaceIndexState::NamespaceIndexWritten;
}

/// `Retry` fired: the outstanding group create, fork or group archive runs again with the next
/// attempt. A group create of an existing root answers as created, and the fork's group writes
/// dedupe on the operation id, so a retry completes the same operation.
fn retry(
	state: &mut State,
	preparation: &Preparation,
	entropy: Entropy,
	now_unix_ms: i64,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let operation_id = namespace.operation_id;
	let group = namespace.group.as_ref().map(|group| group.group);
	match &mut namespace.phase {
		ns::NamespacePhase::NamespaceCreating(creating) => {
			let Some(group) = group else {
				return;
			};
			creating.attempt = creating.attempt.saturating_add(1);
			effects.arm_timer_after(
				ns::NamespaceTimer::NamespaceRetry,
				backoff_ms(&preparation.policy, creating.attempt, entropy),
			);
			effects.execute(Effect::CreateGroup {
				group,
				attempt: creating.attempt,
			});
			namespace
				.start_waiters
				.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
		}
		ns::NamespacePhase::NamespaceForking(forking) => {
			let Some(child) = group else {
				return;
			};
			forking.attempt = forking.attempt.saturating_add(1);
			let effect = Effect::ForkGroup {
				parent: forking.parent,
				child,
				operation_id,
				unix_ms: now_unix_ms,
				attempt: forking.attempt,
			};
			effects.arm_timer_after(
				ns::NamespaceTimer::NamespaceRetry,
				backoff_ms(&preparation.policy, forking.attempt, entropy),
			);
			effects.execute(effect);
			namespace
				.start_waiters
				.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
		}
		ns::NamespacePhase::NamespaceArchiving(archiving) => {
			let Some(group) = group else {
				return;
			};
			archiving.attempt = archiving.attempt.saturating_add(1);
			effects.arm_timer_after(
				ns::NamespaceTimer::NamespaceRetry,
				backoff_ms(&preparation.policy, archiving.attempt, entropy),
			);
			effects.execute(Effect::ArchiveGroup {
				group,
				shards: preparation.event_shards,
				attempt: archiving.attempt,
			});
		}
		ns::NamespacePhase::NamespaceLive
		| ns::NamespacePhase::NamespaceArchived(_)
		| ns::NamespacePhase::NamespaceFailed(_) => {}
	}
}

/// `IndexRetry` fired: the index entry is written again with the next attempt.
fn index_retry(
	state: &mut State,
	attempt: u32,
	preparation: &Preparation,
	entropy: Entropy,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let create_unix_ms = namespace.create_unix_ms;
	let ns::NamespaceIndexState::NamespaceIndexPending(pending) = &mut namespace.index else {
		return;
	};
	if pending.attempt != attempt {
		return;
	}
	pending.attempt = pending.attempt.saturating_add(1);
	let next = pending.attempt;
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceIndexRetry(ns::NamespaceIndexRetry { attempt: next }),
		backoff_ms(&preparation.policy, next, entropy),
	);
	effects.execute(Effect::WriteIndex {
		attempt: next,
		create_unix_ms,
	});
}

/// Legacy: the `BackfillRetry` attempt of the timer a moved namespace's group create outcome
/// arms when it found the fleet reading namespace volumes, which counts as the backfill's first
/// fleet check. A pending backfill's checks otherwise arm attempt 0. Removed in 3.1 with the
/// backfill.
const LEGACY_MOVE_FOUND_FLEET_READY: u32 = 1;

/// Legacy: `BackfillRetry` fired. A pending backfill starts once two checks one start delay
/// apart both find its namespace live and the fleet reading namespace volumes, and then a read of
/// the V2 migrator's state finds no in-flight entry at the legacy placement: the first check, the
/// move's group create outcome (a timer of [`LEGACY_MOVE_FOUND_FLEET_READY`]) or a timer that found
/// the fleet ready, records it in the timer or in `fleet_ready_seen` and checks again after the
/// delay, a check that finds either
/// false clears it, and a check that finds both true reads the migrator's state, with this timer
/// re-armed one start delay later as the read's deadline and the wait's. A running backfill sends its current page again with the next attempt, giving up on
/// the page's first actor once the attempts reach `backfill_max_actor_attempts`; a timer of an
/// earlier attempt, or of a finished backfill, changes nothing. Removed in 3.1 with the backfill.
fn legacy_backfill_retry(
	state: &mut State,
	attempt: u32,
	preparation: &Preparation,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let Some(group) = namespace.group.as_ref() else {
		return;
	};
	let group_id = group.group;
	let index_bucket_count = group.index_bucket_count;
	let policy = &preparation.policy;
	let live = matches!(namespace.phase, ns::NamespacePhase::NamespaceLive);
	match &mut namespace.backfill {
		ns::NamespaceBackfill::NamespaceBackfillPending(pending) => {
			let ready = live && preparation.backfill_fleet_ready;
			let seen = pending.fleet_ready_seen || attempt == LEGACY_MOVE_FOUND_FLEET_READY;
			if !ready || !seen {
				pending.fleet_ready_seen = ready;
				effects.arm_timer_after(
					ns::NamespaceTimer::NamespaceBackfillRetry(ns::NamespaceBackfillRetry {
						attempt: 0,
					}),
					policy.backfill_start_delay_ms,
				);
				return;
			}
			// The second ready check: the migrator's report is awaited now.
			pending.fleet_ready_seen = true;
			effects.arm_timer_after(
				ns::NamespaceTimer::NamespaceBackfillRetry(ns::NamespaceBackfillRetry {
					attempt: 0,
				}),
				policy.backfill_start_delay_ms,
			);
			effects.execute(Effect::LegacyReadMigratorHolds);
		}
		ns::NamespaceBackfill::NamespaceBackfillRunning(running) => {
			if running.attempt != attempt {
				return;
			}
			running.attempt = running.attempt.saturating_add(1);
			running.mover = running.mover.saturating_add(1);
			legacy_send_page(
				group_id,
				index_bucket_count,
				running.cursor.clone(),
				running.attempt,
				running.skipped.clone(),
				running.mover,
				policy,
				effects,
			);
		}
		ns::NamespaceBackfill::NamespaceBackfillDone(_) => {}
	}
}

/// Legacy: whether a pending backfill waits only for the V2 migrator: the fleet was found ready on
/// two checks one start delay apart, and the namespace is live and the fleet still reads
/// namespace volumes now. Removed in 3.1 with the backfill.
fn legacy_awaits_migrator(state: &State, preparation: &Preparation) -> bool {
	let Some(namespace) = &state.0 else {
		return false;
	};
	let live = matches!(namespace.phase, ns::NamespacePhase::NamespaceLive);
	match &namespace.backfill {
		ns::NamespaceBackfill::NamespaceBackfillPending(pending) => {
			live && preparation.backfill_fleet_ready && pending.fleet_ready_seen
		}
		ns::NamespaceBackfill::NamespaceBackfillRunning(_)
		| ns::NamespaceBackfill::NamespaceBackfillDone(_) => false,
	}
}

/// Legacy: a read of the V2 migrator's state reported whether it holds an in-flight entry at the
/// legacy placement for this namespace. A pending backfill that waits only for the migrator
/// starts when it holds none. Waiting matters because an entry begun before the namespace had a
/// group finishes at the legacy placement, and its actor gets the legacy index entry the walk
/// finds only after the activation: a walk started earlier could pass the actor's bucket before
/// the entry lands and end without moving it, after which the namespace's opens stop falling
/// back to the legacy placement and the actor is lost. The migrator finishes such an entry only
/// once the actor's index entry is written, so `held: false` means every actor it activated here
/// is listed. New entries are placed in the group once the namespace has one, so the entries
/// waited for only drain. Removed in 3.1 with the backfill.
fn legacy_migrator_holds(
	state: &mut State,
	held: bool,
	preparation: &Preparation,
	effects: &mut NamespaceEffects,
) {
	if !legacy_awaits_migrator(state, preparation) {
		return;
	}
	if held {
		return;
	}
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let Some(group) = namespace.group.as_ref() else {
		return;
	};
	let group_id = group.group;
	let index_bucket_count = group.index_bucket_count;
	effects.cancel_timer(ns::NamespaceTimer::NamespaceBackfillRetry(
		ns::NamespaceBackfillRetry { attempt: 0 },
	));
	namespace.backfill =
		ns::NamespaceBackfill::NamespaceBackfillRunning(ns::NamespaceBackfillRunning {
			cursor: None,
			attempt: 0,
			skipped: Vec::new(),
			mover: 1,
		});
	effects.execute(Effect::ObserveBackfill {
		phase: BackfillPhase::Running,
	});
	legacy_send_page(
		group_id,
		index_bucket_count,
		None,
		0,
		Vec::new(),
		1,
		&preparation.policy,
		effects,
	);
}

/// Legacy: a backfill page reported. Only a report of the current send, by cursor, attempt and
/// mover token, counts:
/// it records an actor the page gave up on, then moves the walk to the next page, or finishes the
/// backfill. Removed in 3.1 with the backfill.
fn legacy_backfill_outcome(
	state: &mut State,
	outcome: ns::NamespaceBackfillOutcome,
	preparation: &Preparation,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let Some(group) = namespace.group.as_ref() else {
		return;
	};
	let group_id = group.group;
	let index_bucket_count = group.index_bucket_count;
	let ns::NamespaceBackfill::NamespaceBackfillRunning(running) = &mut namespace.backfill else {
		return;
	};
	let current = running.cursor == outcome.from
		&& running.attempt == outcome.attempt
		&& running.mover == outcome.mover;
	if !current {
		return;
	}
	effects.cancel_timer(ns::NamespaceTimer::NamespaceBackfillRetry(
		ns::NamespaceBackfillRetry {
			attempt: running.attempt,
		},
	));
	if let Some(skipped) = outcome.skipped {
		running.skipped.push(skipped.clone());
		effects.execute(Effect::ObserveBackfillSkipped {
			skipped,
			listed: count_u32(running.skipped.len()),
			capacity: count_u32(preparation.policy.backfill_max_skipped),
		});
	}
	let skipped = std::mem::take(&mut running.skipped);
	let mover = running.mover.saturating_add(1);
	match outcome.next {
		ns::NamespaceBackfillNext::NamespaceBackfillNextPage(next) => {
			namespace.backfill =
				ns::NamespaceBackfill::NamespaceBackfillRunning(ns::NamespaceBackfillRunning {
					cursor: Some(next.cursor.clone()),
					attempt: 0,
					skipped: skipped.clone(),
					mover,
				});
			legacy_send_page(
				group_id,
				index_bucket_count,
				Some(next.cursor),
				0,
				skipped,
				mover,
				&preparation.policy,
				effects,
			);
		}
		ns::NamespaceBackfillNext::NamespaceBackfillFinished => {
			namespace.backfill =
				ns::NamespaceBackfill::NamespaceBackfillDone(ns::NamespaceBackfillDone {
					skipped_count: u32::try_from(skipped.len()).unwrap_or(u32::MAX),
					skipped: skipped
						.into_iter()
						.take(LEGACY_BACKFILL_DONE_SKIPPED_LISTED)
						.collect(),
				});
			effects.execute(Effect::ObserveBackfill {
				phase: BackfillPhase::Done,
			});
			if let ns::NamespaceBackfill::NamespaceBackfillDone(done) = &namespace.backfill {
				effects.execute(Effect::ObserveBackfillSkippedListed {
					listed: count_u32(done.skipped.len()),
				});
			}
		}
	}
}

/// Legacy: a backfill page failed at its first actor without giving up on it. Only a failure of
/// the current send, by cursor, attempt and mover token, of a send that refused to give up at the
/// cap is observed, naming the actor the walk now waits on. Changes no state: the page's
/// `BackfillRetry` sends it again. Removed in 3.1 with the backfill.
fn legacy_backfill_page_failed(
	state: &State,
	failed: ns::NamespaceBackfillPageFailed,
	policy: &Policy,
	effects: &mut NamespaceEffects,
) {
	let Some(namespace) = &state.0 else {
		return;
	};
	let ns::NamespaceBackfill::NamespaceBackfillRunning(running) = &namespace.backfill else {
		return;
	};
	let current = running.cursor == failed.from
		&& running.attempt == failed.attempt
		&& running.mover == failed.mover;
	if !current || !legacy_give_up_refused(running.attempt, running.skipped.len(), policy) {
		return;
	}
	effects.execute(Effect::ObserveBackfillSkippedFull {
		actor_name: failed.name.to_string(),
		actor_id: failed.actor_id.to_vec(),
		listed: count_u32(running.skipped.len()),
		capacity: count_u32(policy.backfill_max_skipped),
	});
}

/// Legacy: whether the page send of `attempt`, with `skipped` actors already skipped, reached
/// `backfill_max_actor_attempts` at the `backfill_max_skipped` cap, so it is sent without a
/// give-up and a failure at its first actor fails it. Removed in 3.1 with the backfill.
fn legacy_give_up_refused(attempt: u32, skipped: usize, policy: &Policy) -> bool {
	legacy_attempts_reached(attempt, policy) && skipped >= policy.backfill_max_skipped
}

/// Legacy: whether the page send of `attempt` is the last one allowed before a give-up.
/// Removed in 3.1 with the backfill.
fn legacy_attempts_reached(attempt: u32, policy: &Policy) -> bool {
	attempt.saturating_add(1) >= policy.backfill_max_actor_attempts
}

/// Legacy: a request destroyed the skipped actor `actor_id`, which leaves the backfill's list:
/// a finished backfill's list and count, or a running backfill's list, making room below its
/// cap. Removed in 3.1 with the backfill.
fn legacy_skipped_destroyed(state: &mut State, actor_id: &[u8], effects: &mut NamespaceEffects) {
	let Some(namespace) = &mut state.0 else {
		return;
	};
	let skipped = match &mut namespace.backfill {
		ns::NamespaceBackfill::NamespaceBackfillDone(done) => &mut done.skipped,
		ns::NamespaceBackfill::NamespaceBackfillRunning(running) => &mut running.skipped,
		ns::NamespaceBackfill::NamespaceBackfillPending(_) => return,
	};
	let listed = skipped.len();
	skipped.retain(|skipped| skipped.actor_id.as_ref() != actor_id);
	let remaining = skipped.len();
	if remaining == listed {
		return;
	}
	if let ns::NamespaceBackfill::NamespaceBackfillDone(done) = &mut namespace.backfill {
		let removed = u32::try_from(listed - remaining).unwrap_or(u32::MAX);
		done.skipped_count = done.skipped_count.saturating_sub(removed);
	}
	effects.execute(Effect::ObserveBackfillSkippedListed {
		listed: count_u32(remaining),
	});
}

/// Legacy: the most skipped actors a finished backfill lists, so its record stays bounded; the
/// count goes on past the list. A skipped actor is one no attempt could move, which is rare.
/// Removed in 3.1 with the backfill.
const LEGACY_BACKFILL_DONE_SKIPPED_LISTED: usize = 1_000;

/// Legacy: sends the backfill page at `cursor` under the `BackfillRetry` of `attempt`, passing the
/// actors in `skipped`, as `mover`; the send that reaches `backfill_max_actor_attempts` gives up
/// on an actor it cannot move, unless `skipped` holds `backfill_max_skipped` actors already.
/// Removed in 3.1 with the backfill.
#[allow(
	clippy::too_many_arguments,
	reason = "one field per part of the MovePage effect"
)]
fn legacy_send_page(
	group: GroupId,
	index_bucket_count: u64,
	cursor: Option<ns::NamespaceBackfillPosition>,
	attempt: u32,
	skipped: Vec<ns::NamespaceBackfillSkipped>,
	mover: u64,
	policy: &Policy,
	effects: &mut NamespaceEffects,
) {
	effects.arm_timer_after(
		ns::NamespaceTimer::NamespaceBackfillRetry(ns::NamespaceBackfillRetry { attempt }),
		policy.backfill_page_timeout_ms,
	);
	// At the cap a page never gives up: a failure at its first actor fails it, so the walk
	// waits on that actor and every skipped actor stays listed when the backfill finishes. The
	// page reports that failure, which `BackfillPageFailed` observes with the actor named.
	let give_up = legacy_attempts_reached(attempt, policy)
		&& !legacy_give_up_refused(attempt, skipped.len(), policy);
	effects.execute(Effect::MovePage {
		group,
		index_bucket_count,
		cursor,
		attempt,
		skipped,
		give_up,
		mover,
	});
}

/// A count as the `u32` an effect carries, saturating.
fn count_u32(count: usize) -> u32 {
	u32::try_from(count).unwrap_or(u32::MAX)
}
