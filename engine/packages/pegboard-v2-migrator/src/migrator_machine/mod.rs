//! The V2 migrator state machine: one singleton tenure moving one V2 namespace's actors into V3.
//!
//! **Durable state.** The `[V2_MIGRATOR]` volume holds the pass entry (cursors and counts) and one
//! entry per actor in flight. Every write is fenced by the tenure. Each transition stores its new
//! phase before it issues the effect the phase implies, so a new tenure that loads the entries
//! reissues the same idempotent request with the same arguments. Completed actors leave no entry:
//! the active V3 state and V2's parked record are the record.
//!
//! **Per actor.** `Copying` (V3 actor held as `MigratingFromV2`, export imported) ->
//! `CuttingOver` (V2 stops waking it) -> `Confirming` (after the confirm delay, V2 parks it) ->
//! `Activating` (V3 serves it). `Discarding` abandons an attempt before V2 confirmed it, which is
//! safe because V3 never served the actor.
//!
//! **Failover.** A loaded `Copying` entry is discarded and retried by a later pass; `CuttingOver`,
//! `Activating` and `Discarding` reissue their effect; `Confirming` waits for `confirmAt`.
//!
//! **Placement.** An admission places its entry where a fresh create in the target namespace goes,
//! in the namespace's volume group or, while it has none, at the legacy unprefixed placement, and
//! persists that placement with the entry; every effect addresses it, whatever the namespace's
//! standing says later. Entries that 3.0.2 and 3.0.3 wrote are all legacy-placed. A legacy-placed
//! entry whose namespace has a group by now is begun again in the group when V2 has not been
//! touched yet: a loaded `Copying` or `Reserving` entry is discarded at the legacy placement and a
//! later pass admits the actor in the group, and a `Copying` entry whose copy finds the group
//! before its begin is moved there in place. Once V2 was cut over the entry finishes at the legacy
//! placement, and the namespace's backfill, which does not start while the migrator holds a
//! legacy-placed entry, moves the activated actor into the group.
//!
//! **Listing.** One candidates page at a time, finishing each configured V2 datacenter's cursor
//! before starting the next. Pages are full size, since most candidates are not selected. Selected
//! candidates are admitted until the free slots run out; the cursor then moves to the last
//! candidate handled, using the per-candidate cursor V2 returns, so the next page starts with the
//! first one that did not fit. The cursor and the new entries are stored in the same write.
//!
//! **Copy budget.** Besides `max_in_flight` and `max_actors_per_sec`, admissions are paced by
//! bytes: each admitted actor is charged its V2 size at `max_copy_bytes_per_sec`, and a selected
//! candidate waits, like one without a free slot, while the charged bytes run more than
//! [`COPY_BURST_MS`] ahead. Copies are one opaque effect each, so the budget is spent at admission
//! rather than metered while a copy streams.
//!
//! **Requests.** An operator can ask for one actor by id, whatever `migrate_ratio` says. The core
//! reads the actor from V2, admits it under every selection rule but the share, and replies once
//! its entry is stored. One request is answered at a time. Requested actors do not wait for
//! `max_in_flight` slots, which the automatic listing refills the moment they free; they have
//! their own allowance of [`MAX_REQUESTED_IN_FLIGHT`] and count as taken slots for the listing.
//! The entries can therefore exceed `max_in_flight` by that allowance plus the one read.
use anyhow::ensure;
use rivet_env::Clocks;
use rivet_state_machine::Entropy;
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use rivet_util::Id;
use std::collections::BTreeSet;

pub mod effect;
pub mod event;
pub mod executor;
pub mod snapshot;
pub mod state;
pub mod types;

use effect::Effect;
use event::Event;
use state::*;
use types::*;

pub type Effects = rivet_state_machine::Effects<Effect, Timer>;

/// Timeout of one effect other than a copy.
const IO_TIMEOUT_MS: u64 = 60_000;
/// Timeout of one copy, long enough for a 10 GiB actor at a modest rate.
const COPY_TIMEOUT_MS: u64 = 60 * 60 * 1000;
const RETRY_BASE_MS: u64 = 1_000;
const RETRY_MAX_MS: u64 = 30_000;
/// Largest candidates page.
const PAGE_LIMIT: u32 = 200;
/// Timeout of an operator request's candidate read. The operator's UPS request waits at most 30
/// s, so the read fails in time for the reply to reach it.
const CANDIDATE_TIMEOUT_MS: u64 = 20_000;
/// How far ahead of now the charged copy bytes may run while another actor is still admitted:
/// one second of `max_copy_bytes_per_sec`. Small actors are admitted in bursts up to it; an actor
/// larger than it is admitted alone once the earlier charges are nearly paid.
pub const COPY_BURST_MS: i64 = 1_000;
/// Activation attempts that may find no actor at the entry's placement before the entry finishes
/// as needing recovery. An authoritative missing or deleted volume does not come back, so the
/// retries only cover a misread; without a bound the entry would hold its slot, and at the legacy
/// placement its namespace's backfill, forever.
pub const HOLD_VANISHED_MAX_ATTEMPTS: u32 = 5;
/// Operator-requested actors in flight at once, beyond `max_in_flight`. A script that migrates one
/// actor at a time and waits for each uses one.
pub const MAX_REQUESTED_IN_FLIGHT: usize = 4;

pub struct MigratorStateMachine;

impl rivet_state_machine::StateMachineState for State {}
impl rivet_state_machine::Snapshot for snapshot::Snapshot {}

impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::LoadStateResponse { .. } => "load_state_response",
			Event::WriteStateResponse { .. } => "write_state_response",
			Event::ListCandidatesResponse { .. } => "list_candidates_response",
			Event::CopyResponse { .. } => "copy_response",
			Event::ReadReservationResponse { .. } => "read_reservation_response",
			Event::CutoverResponse { .. } => "cutover_response",
			Event::ConfirmResponse { .. } => "confirm_response",
			Event::ActivateResponse { .. } => "activate_response",
			Event::DiscardResponse { .. } => "discard_response",
			Event::MigrateActorRequest { .. } => "migrate_actor_request",
			Event::ReadCandidateResponse { .. } => "read_candidate_response",
			Event::SettingsChanged { .. } => "settings_changed",
			Event::StopRequest => "stop_request",
		}
	}
}

impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::LoadStateRequest { .. } => "load_state_request",
			Effect::WriteStateRequest { .. } => "write_state_request",
			Effect::ListCandidatesRequest { .. } => "list_candidates_request",
			Effect::CopyRequest { .. } => "copy_request",
			Effect::ReadReservationRequest { .. } => "read_reservation_request",
			Effect::CutoverRequest { .. } => "cutover_request",
			Effect::ConfirmRequest { .. } => "confirm_request",
			Effect::ActivateRequest { .. } => "activate_request",
			Effect::DiscardRequest { .. } => "discard_request",
			Effect::ReadCandidateRequest { .. } => "read_candidate_request",
			Effect::MigrateActorResponse { .. } => "migrate_actor_response",
			Effect::RecordResult { .. } => "record_result",
			Effect::RecordEffectFailure { .. } => "record_effect_failure",
			Effect::RecordCopyAdmitted { .. } => "record_copy_admitted",
			Effect::ReportPassEnd { .. } => "report_pass_end",
			Effect::NotifyLegacySettled { .. } => "notify_legacy_settled",
		}
	}
}

impl State {
	pub fn new(tenure: u64, settings: Settings) -> Self {
		State {
			tenure,
			settings,
			phase: Phase::LoadBackoff,
			pass: None,
			pass_dirty: false,
			listing: Listing::Idle,
			entries: Default::default(),
			persist: Persist::Idle,
			persist_failures: 0,
			next_op: 0,
			next_admit_unix_ms: 0,
			copy_budget_unix_ms: 0,
			requested: None,
			requested_entries: Default::default(),
		}
	}

	fn next_op(&mut self) -> OpId {
		self.next_op += 1;
		OpId(self.next_op)
	}
}

impl EphemeralStateMachine for MigratorStateMachine {
	type Slot = rivet_state_machine::SingleSlot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = snapshot::Snapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "migrate_from_v2";

	fn timer_name(timer: &Timer) -> &'static str {
		match timer {
			Timer::Load => "load",
			Timer::Persist => "persist",
			Timer::Listing => "listing",
			Timer::Pass => "pass",
			Timer::Entry(_) => "entry",
			Timer::Io(_) => "io",
		}
	}

	fn start(state: &mut State, _now: Clocks, _entropy: Entropy, out: &mut Effects) {
		load(state, out);
	}

	fn on_event(state: &mut State, event: EphemeralStateMachineEvent<Self>, out: &mut Effects) {
		let now = event.now;
		if let Phase::Stopped = state.phase {
			return;
		}

		match event.body {
			RuntimeEvent::Input(Event::StopRequest) => {
				state.phase = Phase::Stopped;
				return;
			}
			RuntimeEvent::Input(Event::SettingsChanged { settings }) => {
				on_settings(state, settings);
			}
			RuntimeEvent::Input(Event::MigrateActorRequest {
				request,
				v2_actor_id,
			}) => on_migrate_actor_request(state, request, v2_actor_id, out),
			RuntimeEvent::Input(Event::ReadCandidateResponse { op, result }) => {
				on_candidate(state, op, result, now, out);
			}
			RuntimeEvent::Input(Event::LoadStateResponse { op, result }) => {
				on_loaded(state, op, result, now, out);
			}
			RuntimeEvent::Timer(Timer::Load) => {
				if let Phase::LoadBackoff = state.phase {
					load(state, out);
				}
				return;
			}
			RuntimeEvent::Input(Event::WriteStateResponse { op, result }) => {
				on_written(state, op, result, now, out);
			}
			RuntimeEvent::Timer(Timer::Persist) => {
				if let Persist::Backoff = state.persist {
					state.persist = Persist::Idle;
				}
			}
			RuntimeEvent::Input(Event::ListCandidatesResponse {
				op,
				datacenter_label,
				result,
			}) => on_page(state, op, datacenter_label, result, now, out),
			RuntimeEvent::Timer(Timer::Listing) => {
				if let Listing::Backoff = state.listing {
					state.listing = Listing::Idle;
				}
			}
			RuntimeEvent::Timer(Timer::Pass) => {
				if let Listing::PassDone = state.listing {
					start_pass(state, now);
					state.listing = Listing::Idle;
				}
			}
			RuntimeEvent::Timer(Timer::Entry(v2_actor_id)) => {
				if let Some(entry) = state.entries.get_mut(&v2_actor_id)
					&& let Step::Waiting = entry.step
				{
					entry.step = Step::Ready;
				}
			}
			RuntimeEvent::Timer(Timer::Io(op)) => on_io_timeout(state, op, now, out),
			RuntimeEvent::Input(Event::CopyResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					on_copied(state, v2_actor_id, result);
				}
			}
			RuntimeEvent::Input(Event::ReadReservationResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					on_reservation(state, v2_actor_id, result, now, out);
				}
			}
			RuntimeEvent::Input(Event::CutoverResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					on_cutover(state, v2_actor_id, result, now, out);
				}
			}
			RuntimeEvent::Input(Event::ConfirmResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					on_confirm(state, v2_actor_id, result, out);
				}
			}
			RuntimeEvent::Input(Event::ActivateResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					match result {
						ActivateResult::Applied => {
							finish(state, v2_actor_id, Outcome::Confirmed, Reason::None)
						}
						// The actor is activated, but its legacy index entry is not written yet.
						// The entry stays, so the namespace's backfill keeps waiting, and reads the
						// actor again after a backoff; the actor state machine retries the write.
						ActivateResult::IndexPending => {
							if let Some(entry) = state.entries.get_mut(&v2_actor_id) {
								entry.failures += 1;
								tracing::debug!(
									%v2_actor_id,
									"the activated actor's legacy index entry is not written yet; reading it again"
								);
								entry.step = Step::Waiting;
								out.arm_timer_after(
									Timer::Entry(v2_actor_id),
									backoff_ms(entry.failures),
								);
							}
						}
						// Another migration holds the V3 actor. That answer comes from a read of the
						// actor's state and no retry changes it, so the entry finishes and frees its
						// slot. V2's parked record still names the V3 actor for the operator.
						ActivateResult::Refused => {
							if let Some(entry) = state.entries.get(&v2_actor_id) {
								tracing::error!(
									%v2_actor_id,
									v3_actor_id = ?Id::from_slice(&entry.record.v3_actor_id).ok(),
									migration_id = entry.record.migration_id,
									"V2 parked this actor but another migration holds its V3 state, so it cannot be activated; it is served by neither until an operator resolves it"
								);
							}
							finish(
								state,
								v2_actor_id,
								Outcome::NeedsRecovery,
								Reason::ActivateRefused,
							);
						}
						ActivateResult::Vanished(error) => {
							on_activation_vanished(state, v2_actor_id, &error, out)
						}
						ActivateResult::Failed(error) => {
							retry_later(state, v2_actor_id, &error, out)
						}
					}
				}
			}
			RuntimeEvent::Input(Event::DiscardResponse {
				op,
				v2_actor_id,
				result,
			}) => {
				if take_running(state, v2_actor_id, op, out) {
					match result {
						V3Result::Applied | V3Result::Refused => {
							let (outcome, reason) = state
								.entries
								.get(&v2_actor_id)
								.map(discard_outcome)
								.unwrap_or((Outcome::Failed, Reason::None));
							finish(state, v2_actor_id, outcome, reason);
						}
						V3Result::Failed(error) => retry_later(state, v2_actor_id, &error, out),
					}
				}
			}
		}

		if let Phase::Running = state.phase {
			dispatch(state, now, out);
			answer_stored_request(state, out);
		}
	}

	fn snapshot(state: &State) -> snapshot::Snapshot {
		snapshot::build(state)
	}

	fn debug_validate(state: &State) -> anyhow::Result<()> {
		let persisting = state
			.entries
			.values()
			.filter(|entry| matches!(entry.step, Step::Persisting(_)))
			.count();
		ensure!(
			persisting == 0 || matches!(state.persist, Persist::Writing(_)),
			"entries are persisting without a write in flight"
		);
		Ok(())
	}
}

fn load(state: &mut State, out: &mut Effects) {
	let op = state.next_op();
	state.phase = Phase::Loading { op };
	out.arm_timer_after(Timer::Io(op), IO_TIMEOUT_MS);
	out.execute(Effect::LoadStateRequest {
		op,
		tenure: state.tenure,
	});
}

fn on_loaded(state: &mut State, op: OpId, result: LoadResult, now: Clocks, out: &mut Effects) {
	if state.phase != (Phase::Loading { op }) {
		return;
	}
	out.cancel_timer(Timer::Io(op));

	let (pass, in_flight) = match result {
		LoadResult::Loaded { pass, in_flight } => (pass, in_flight),
		LoadResult::Fenced => {
			tracing::warn!(
				tenure = state.tenure,
				"a newer migrator tenure claimed the state; stopping"
			);
			state.phase = Phase::Stopped;
			return;
		}
		LoadResult::Failed(error) => {
			tracing::warn!(%error, "loading the migration state failed; retrying");
			state.phase = Phase::LoadBackoff;
			out.arm_timer_after(Timer::Load, RETRY_BASE_MS * 5);
			return;
		}
	};

	state.phase = Phase::Running;
	state.pass = pass;
	if state.pass.is_none() {
		start_pass(state, now);
	}
	if let Some(pass) = &state.pass
		&& let Some(ended_at) = pass.ended_at
	{
		state.listing = Listing::PassDone;
		let start_at = ended_at.saturating_add(state.settings.pass_interval_ms as i64);
		out.arm_timer_at(Timer::Pass, boot_at(now, start_at));
	}

	for record in in_flight {
		let Ok(v2_actor_id) = Id::from_slice(&record.candidate.v2_actor_id) else {
			tracing::error!("skipping an in-flight entry with an invalid actor id");
			continue;
		};
		let mut entry = Entry {
			record,
			step: Step::Ready,
			failures: 0,
			vanished: 0,
			placement_moved: false,
			finished: None,
		};
		let placement_moved = entry.record.group.is_none() && state.settings.target_group.is_some();
		match &entry.record.phase {
			// The copy restarts on a later pass. Reopening the volume fences the old writer.
			wire::MigrateFromV2Phase::MigrateFromV2Copying => {
				entry.record.phase = wire::MigrateFromV2Phase::MigrateFromV2Discarding(
					wire::MigrateFromV2Discarding {
						reason: wire::MigrateFromV2DiscardReason::Failover,
					},
				);
				entry.step = Step::Dirty;
			}
			wire::MigrateFromV2Phase::MigrateFromV2Confirming(confirming) => {
				entry.step = Step::Waiting;
				out.arm_timer_at(
					Timer::Entry(v2_actor_id),
					boot_at(now, confirming.confirm_at),
				);
			}
			// A legacy-placed hold of a namespace that has a group now is begun again in the
			// group: V2 was not cut over yet, so discarding the hold loses nothing, and a later
			// pass admits the actor at the group's placement.
			wire::MigrateFromV2Phase::MigrateFromV2Reserving if placement_moved => {
				tracing::info!(
					%v2_actor_id,
					"a legacy-placed migration of a namespace that has a volume group now is discarded and begun again in the group"
				);
				entry.record.phase = wire::MigrateFromV2Phase::MigrateFromV2Discarding(
					wire::MigrateFromV2Discarding {
						reason: wire::MigrateFromV2DiscardReason::Failover,
					},
				);
				entry.step = Step::Dirty;
			}
			// V2 was cut over or the hold is being discarded, so the entry finishes where it was
			// placed. The backfill moves an actor activated at the legacy placement.
			wire::MigrateFromV2Phase::MigrateFromV2Reserving
			| wire::MigrateFromV2Phase::MigrateFromV2CuttingOver(_)
			| wire::MigrateFromV2Phase::MigrateFromV2Activating(_)
			| wire::MigrateFromV2Phase::MigrateFromV2Discarding(_) => {}
		}
		state.entries.insert(v2_actor_id, entry);
	}
}

fn start_pass(state: &mut State, now: Clocks) {
	let previous = state.pass.take();
	let pass = previous.as_ref().map_or(1, |pass| pass.pass + 1);
	let next_migration_seq = previous.as_ref().map_or(0, |pass| pass.next_migration_seq);
	state.pass = Some(wire::MigrateFromV2Pass {
		pass,
		started_at: now.unix_ms,
		cursors: state
			.settings
			.datacenter_labels
			.iter()
			.map(|label| wire::MigrateFromV2Cursor {
				datacenter_label: *label,
				after: None,
				done: false,
			})
			.collect(),
		counts: empty_counts(),
		ended_at: None,
		next_migration_seq,
	});
	state.pass_dirty = true;
}

fn empty_counts() -> wire::MigrateFromV2Counts {
	wire::MigrateFromV2Counts {
		listed: 0,
		selected: 0,
		confirmed: 0,
		discarded: 0,
		skipped: 0,
		duplicate_key: 0,
		already_migrated: 0,
		not_selected: 0,
		too_large: 0,
		needs_recovery: 0,
	}
}

/// Converts a unix-ms deadline into the boot clock the runner's timers use.
fn boot_at(now: Clocks, unix_ms: i64) -> u64 {
	let delay = unix_ms.saturating_sub(now.unix_ms).max(0) as u64;
	now.boot_ms.saturating_add(delay)
}

fn on_written(state: &mut State, op: OpId, result: WriteResult, now: Clocks, out: &mut Effects) {
	let persist = match std::mem::replace(&mut state.persist, Persist::Idle) {
		Persist::Writing(persist) if persist.op == op => persist,
		other => {
			state.persist = other;
			return;
		}
	};
	out.cancel_timer(Timer::Io(op));

	match result {
		WriteResult::Written => {
			state.persist_failures = 0;
			// The namespaces whose backfill may read again, because this write stored the end of
			// one of their legacy-placed entries.
			let mut legacy_settled = BTreeSet::new();
			for v2_actor_id in &persist.deletes {
				state.requested_entries.remove(v2_actor_id);
				let Some(entry) = state.entries.remove(v2_actor_id) else {
					continue;
				};
				if entry.record.group.is_none() {
					legacy_settled.insert(entry_namespace(&entry.record));
				}
				if let Some((outcome, reason)) = entry.finished {
					out.execute(Effect::RecordResult { outcome, reason });
				}
			}
			for v2_actor_id in &persist.puts {
				let Some(entry) = state.entries.get_mut(v2_actor_id) else {
					continue;
				};
				// The write stored the entry's move into the group, so the migrator no longer
				// holds it at the legacy placement.
				if std::mem::take(&mut entry.placement_moved) {
					legacy_settled.insert(entry_namespace(&entry.record));
				}
				if entry.step != Step::Persisting(op) {
					continue;
				}
				entry.step = match &entry.record.phase {
					wire::MigrateFromV2Phase::MigrateFromV2Confirming(confirming)
						if confirming.confirm_at > now.unix_ms =>
					{
						out.arm_timer_at(
							Timer::Entry(*v2_actor_id),
							boot_at(now, confirming.confirm_at),
						);
						Step::Waiting
					}
					_ => Step::Ready,
				};
			}
			for namespace in legacy_settled {
				out.execute(Effect::NotifyLegacySettled { namespace });
			}
		}
		WriteResult::Fenced => {
			tracing::warn!(
				tenure = state.tenure,
				"a newer migrator tenure fenced this one; stopping"
			);
			state.phase = Phase::Stopped;
		}
		WriteResult::Refused {
			retry_after_ms,
			error,
		} => {
			state.persist_failures += 1;
			// Depot counts its refusals, so each one is not worth a warning; the backoff below
			// keeps the resend rate low while the storage host sheds load.
			tracing::debug!(
				%error,
				retry_after_ms,
				failures = state.persist_failures,
				"the storage host refused the migration state write; retrying after a backoff"
			);
			let backoff_ms =
				backoff_ms(state.persist_failures).max(retry_after_ms.min(RETRY_MAX_MS));
			back_off_persist(state, &persist, backoff_ms, out);
		}
		WriteResult::Failed(error) => {
			state.persist_failures += 1;
			tracing::warn!(
				%error,
				failures = state.persist_failures,
				"writing the migration state failed; retrying after a backoff"
			);
			let backoff_ms = backoff_ms(state.persist_failures);
			back_off_persist(state, &persist, backoff_ms, out);
		}
	}
}

/// Returns a failed write's entries to `Dirty` and holds every write until the `Persist` timer
/// fires after `backoff_ms`.
fn back_off_persist(state: &mut State, persist: &PersistOp, backoff_ms: u64, out: &mut Effects) {
	for v2_actor_id in persist.puts.iter().chain(persist.deletes.iter()) {
		if let Some(entry) = state.entries.get_mut(v2_actor_id)
			&& entry.step == Step::Persisting(persist.op)
		{
			entry.step = Step::Dirty;
		}
	}
	state.pass_dirty = true;
	state.persist = Persist::Backoff;
	out.arm_timer_after(Timer::Persist, backoff_ms);
}

fn backoff_ms(failures: u32) -> u64 {
	RETRY_BASE_MS
		.saturating_mul(1 << failures.min(5))
		.min(RETRY_MAX_MS)
}

/// Selects a listed candidate, or returns why it is not admitted.
pub fn select(
	candidate: &CandidateFacts,
	settings: &Settings,
	now_unix_ms: i64,
) -> Result<(), Reason> {
	select_with(candidate, settings, now_unix_ms, Share::Applied)
}

/// Selects an actor an operator asked for by id: every rule but the share applies.
pub fn select_requested(
	candidate: &CandidateFacts,
	settings: &Settings,
	now_unix_ms: i64,
) -> Result<(), Reason> {
	select_with(candidate, settings, now_unix_ms, Share::Ignored)
}

/// Whether selection checks the stable-hash share of `migrate_ratio`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Share {
	Applied,
	Ignored,
}

fn select_with(
	candidate: &CandidateFacts,
	settings: &Settings,
	now_unix_ms: i64,
	share: Share,
) -> Result<(), Reason> {
	if candidate.parked {
		return Err(Reason::AlreadyMigrated);
	}
	// A V2 key that does not translate to exactly one canonical key could collide with another
	// actor's, so it stays in V2.
	if candidate
		.key
		.as_deref()
		.is_some_and(|key| v3_key(key).is_none())
	{
		return Err(Reason::KeyUntranslatable);
	}
	if candidate.destroy_ts.is_some() {
		return Err(Reason::Destroyed);
	}
	if share == Share::Applied && !in_share(candidate.actor_id, settings) {
		return Err(Reason::NotSelected);
	}
	if candidate.version == 1 {
		return Err(Reason::ActorV1);
	}
	if candidate.sqlite_v1 {
		return Err(Reason::SqliteV1);
	}
	if copy_bytes(candidate) > settings.max_copy_bytes {
		return Err(Reason::TooLarge);
	}
	if candidate.hibernating {
		return Err(Reason::Hibernating);
	}
	let Some(sleep_ts) = candidate.sleep_ts else {
		return Err(Reason::RecentlyAwake);
	};
	if candidate.allocated || now_unix_ms.saturating_sub(sleep_ts) < settings.min_sleep_ms as i64 {
		return Err(Reason::RecentlyAwake);
	}
	if let Some(deadline) = candidate.wake_deadline_ts
		&& deadline < now_unix_ms.saturating_add(settings.wake_horizon_ms as i64)
	{
		return Err(Reason::WakeSoon);
	}
	Ok(())
}

/// The bytes a copy of the candidate would move, from V2's SQLite head and KV estimate.
pub fn copy_bytes(candidate: &CandidateFacts) -> u64 {
	let sqlite_bytes = u64::from(candidate.db_size_pages) * crate::SQLITE_PAGE_SIZE as u64;
	sqlite_bytes.saturating_add(candidate.kv_bytes_estimate)
}

/// Whether the actor falls in share **a**: `xxh3_64(migrate_seed, actor id) / 2^64 < a`. The hash is
/// stable, so raising **a** only adds actors.
pub fn in_share(actor_id: Id, settings: &Settings) -> bool {
	let hash = xxhash_rust::xxh3::xxh3_64_with_seed(&actor_id.as_bytes(), settings.migrate_seed);
	let threshold = (u128::from(settings.migrate_ratio_ppm) << 64) / 1_000_000;
	u128::from(hash) < threshold
}

/// The V3 id of a migrated actor: a keyless actor keeps its V2 id, since V3 placement derives
/// from the id; a keyed actor takes V3's key-derived id, so its key, state and database share
/// one placement. The id is the one a fresh create in the target namespace takes today; an
/// in-flight entry keeps the id it persisted.
pub fn v3_actor_id(candidate: &CandidateFacts, settings: &Settings) -> Id {
	placed_v3_actor_id(
		candidate.actor_id,
		&candidate.name,
		candidate.key.as_deref(),
		settings.target_group.is_some(),
		settings,
	)
}

/// The V3 id of the V2 actor `v2_actor_id`, named `name` with the V2 slash key `key`, placed in
/// the target namespace's group when `grouped`, at the legacy unprefixed placement otherwise.
fn placed_v3_actor_id(
	v2_actor_id: Id,
	name: &str,
	key: Option<&str>,
	grouped: bool,
	settings: &Settings,
) -> Id {
	// A key without a canonical translation is never selected, so it keeps the V2 id here.
	match key.and_then(v3_key) {
		Some(key) if grouped => {
			pegboard::actor_key::keyed_actor_id(settings.v3_datacenter_label, name, &key)
		}
		Some(key) => pegboard::actor_key::legacy_keyed_actor_id(
			settings.v3_datacenter_label,
			&settings.target_namespace,
			name,
			&key,
		),
		None => v2_actor_id,
	}
}

/// Takes new settings. A namespace gains its volume group once and never loses it, so settings
/// read from a standing older than one this tenure already saw grouped keep the group.
fn on_settings(state: &mut State, mut settings: Settings) {
	let same_namespace = settings.target_namespace == state.settings.target_namespace;
	if same_namespace && settings.target_group.is_none() {
		settings.target_group = state.settings.target_group;
	}
	state.settings = settings;
}

/// The canonical V3 key of a key V2 stored in its slash form, or `None` when the V2 key is not
/// exactly what the slash encoding writes for its parts. Distinct translatable V2 keys always
/// have distinct V3 keys.
pub fn v3_key(v2_key: &str) -> Option<String> {
	rivetkit_shared_types::actor_key::from_slash(v2_key).ok()
}

/// A migration id is the pass entry's sequence, which every tenure persists before it hands one
/// out, so ids order by when they were handed out across tenures. A begin with a newer id takes
/// over a hold that a stale begin left behind. Recovery uses [`RECOVERY_MIGRATION_ID`], which no
/// migrator id reaches, so a migrator begin never takes over a recovery in progress.
fn migration_id(seq: u64) -> u64 {
	seq.min(RECOVERY_MIGRATION_ID - 1)
}

/// The migration id a recovery holds a V3 actor under while it hands the actor back to V2.
pub const RECOVERY_MIGRATION_ID: u64 = u64::MAX;

fn on_page(
	state: &mut State,
	op: OpId,
	datacenter_label: u16,
	result: ListResult,
	now: Clocks,
	out: &mut Effects,
) {
	if state.listing
		!= (Listing::Requesting {
			op,
			datacenter_label,
		}) {
		return;
	}
	out.cancel_timer(Timer::Io(op));

	let (candidates, next) = match result {
		ListResult::Page { candidates, next } => (candidates, next),
		ListResult::Failed(error) => {
			tracing::warn!(%error, datacenter_label, "listing V2 candidates failed; retrying");
			state.listing = Listing::Backoff;
			out.arm_timer_after(Timer::Listing, RETRY_BASE_MS * 5);
			return;
		}
	};

	let free = free_slots(state);
	let Some(pass) = state.pass.as_mut() else {
		state.listing = Listing::Idle;
		return;
	};
	let mut admitted = 0u64;
	// The cursor of the last candidate handled, and whether a selected one did not fit.
	let mut last_cursor = None;
	let mut filled = false;
	for listed in candidates {
		let candidate = listed.facts;
		let selection = match select(&candidate, &state.settings, now.unix_ms) {
			Ok(()) if state.entries.contains_key(&candidate.actor_id) => Err(Reason::InFlight),
			selection => selection,
		};
		// A selected candidate with no free slot or no copy budget is left for the next page,
		// which starts at it.
		let budget_spent = state.copy_budget_unix_ms > now.unix_ms.saturating_add(COPY_BURST_MS);
		if selection.is_ok() && (admitted >= u64::from(free) || budget_spent) {
			filled = true;
			break;
		}
		last_cursor = Some(listed.cursor);
		pass.counts.listed += 1;
		match selection {
			Ok(()) => {}
			Err(Reason::AlreadyMigrated) => {
				pass.counts.already_migrated += 1;
				continue;
			}
			Err(Reason::NotSelected) => {
				pass.counts.not_selected += 1;
				continue;
			}
			Err(Reason::InFlight) => continue,
			Err(Reason::TooLarge) => {
				pass.counts.skipped += 1;
				pass.counts.too_large += 1;
				tracing::info!(
					v2_actor_id = %candidate.actor_id,
					db_size_pages = candidate.db_size_pages,
					kv_bytes_estimate = candidate.kv_bytes_estimate,
					max_copy_bytes = state.settings.max_copy_bytes,
					"skipping a V2 actor larger than max_copy_bytes; raise it to migrate the actor"
				);
				out.execute(Effect::RecordResult {
					outcome: Outcome::Skipped,
					reason: Reason::TooLarge,
				});
				continue;
			}
			Err(reason) => {
				pass.counts.skipped += 1;
				let outcome = match reason {
					Reason::Destroyed => Outcome::Destroyed,
					_ => Outcome::Skipped,
				};
				out.execute(Effect::RecordResult { outcome, reason });
				continue;
			}
		}

		pass.counts.selected += 1;
		let seq = pass.next_migration_seq;
		pass.next_migration_seq += 1;
		let entry = new_entry(&candidate, datacenter_label, seq, &state.settings, now);
		state.entries.insert(candidate.actor_id, entry);
		let bytes = copy_bytes(&candidate);
		charge_copy(&mut state.copy_budget_unix_ms, &state.settings, bytes, now);
		out.execute(Effect::RecordCopyAdmitted { bytes });
		admitted += 1;
	}

	if let Some(cursor) = pass
		.cursors
		.iter_mut()
		.find(|cursor| cursor.datacenter_label == datacenter_label)
	{
		if filled {
			// Nothing was handled when the first candidate already did not fit, so the cursor
			// stays where it was.
			if let Some(last_cursor) = last_cursor {
				cursor.after = Some(last_cursor.into());
			}
			cursor.done = false;
		} else {
			cursor.done = next.is_none();
			cursor.after = next.map(Into::into);
		}
	}
	state.pass_dirty = true;

	// Pace admissions at `max_actors_per_sec`. The page's whole budget is divided at once so a
	// rate above 1000 per second does not round each actor's share down to nothing.
	let max_actors_per_sec = u64::from(state.settings.max_actors_per_sec.max(1));
	let page_budget_ms = admitted.saturating_mul(1_000) / max_actors_per_sec;
	state.next_admit_unix_ms = now
		.unix_ms
		.max(state.next_admit_unix_ms)
		.saturating_add(i64::try_from(page_budget_ms).unwrap_or(i64::MAX));
	state.listing = Listing::Idle;
}

/// Charges an admitted actor's copy `bytes` against the budget at `max_copy_bytes_per_sec`.
fn charge_copy(copy_budget_unix_ms: &mut i64, settings: &Settings, bytes: u64, now: Clocks) {
	let rate = u128::from(settings.max_copy_bytes_per_sec.max(1));
	let cost_ms = (u128::from(bytes) * 1_000).div_ceil(rate);
	let cost_ms = i64::try_from(cost_ms).unwrap_or(i64::MAX);
	*copy_budget_unix_ms = (*copy_budget_unix_ms)
		.max(now.unix_ms)
		.saturating_add(cost_ms);
}

/// Slots the automatic listing may fill: `max_in_flight` less every entry, operator-requested ones
/// included, and less the slot an operator request's candidate read holds until it answers.
fn free_slots(state: &State) -> u32 {
	let reading_candidate = matches!(
		state.requested,
		Some(Requested {
			step: RequestedStep::ReadingCandidate { .. },
			..
		})
	);
	state
		.settings
		.max_in_flight
		.saturating_sub(state.entries.len() as u32)
		.saturating_sub(u32::from(reading_candidate))
}

/// A `Copying` entry for a selected candidate under migration sequence `seq`. It is `Dirty`, so the
/// next write stores it before its copy starts.
fn new_entry(
	candidate: &CandidateFacts,
	datacenter_label: u16,
	seq: u64,
	settings: &Settings,
	now: Clocks,
) -> Entry {
	let v3_actor_id = v3_actor_id(candidate, settings);
	let record = wire::MigrateFromV2InFlight {
		migration_id: migration_id(seq),
		v3_actor_id: v3_actor_id.as_bytes().into(),
		candidate: wire::MigrateFromV2Candidate {
			v2_actor_id: candidate.actor_id.as_bytes().into(),
			datacenter_label,
			name: candidate.name.clone().into(),
			key: candidate.key.clone().map(Into::into),
			pool_name: candidate.pool_name.clone().into(),
			create_ts: candidate.create_ts,
			generation: candidate.generation,
			sleep_ts: candidate.sleep_ts.unwrap_or_default(),
			wake_deadline_ts: candidate.wake_deadline_ts,
		},
		started_at: now.unix_ms,
		phase: wire::MigrateFromV2Phase::MigrateFromV2Copying,
		group: settings.target_group,
		namespace: Some(settings.target_namespace.clone().into()),
	};
	Entry {
		record,
		step: Step::Dirty,
		failures: 0,
		vanished: 0,
		placement_moved: false,
		finished: None,
	}
}

/// Starts answering an operator's request to migrate `v2_actor_id`, or refuses it.
fn on_migrate_actor_request(
	state: &mut State,
	request: RequestId,
	v2_actor_id: Id,
	out: &mut Effects,
) {
	let refusal = if state.phase != Phase::Running {
		Some(Unavailable::Loading)
	} else if state.requested.is_some() {
		Some(Unavailable::RequestPending)
	} else {
		None
	};
	if let Some(refusal) = refusal {
		out.execute(Effect::MigrateActorResponse {
			request,
			result: MigrateActorResult::Unavailable(refusal),
		});
		return;
	}

	// An actor already in flight is answered once its entry is stored, so a retry after a lost
	// reply gets the same answer.
	if let Some(entry) = state.entries.get(&v2_actor_id) {
		let Ok(v3_actor_id) = Id::from_slice(&entry.record.v3_actor_id) else {
			out.execute(Effect::MigrateActorResponse {
				request,
				result: MigrateActorResult::Failed(
					"the in-flight entry has an invalid V3 id".into(),
				),
			});
			return;
		};
		state.requested = Some(Requested {
			request,
			v2_actor_id,
			step: RequestedStep::Storing {
				v3_actor_id,
				migration_id: entry.record.migration_id,
			},
		});
		return;
	}

	let refusal = if !state
		.settings
		.datacenter_labels
		.contains(&v2_actor_id.label())
	{
		Some(Unavailable::DatacenterNotConfigured)
	} else if state.requested_entries.len() >= MAX_REQUESTED_IN_FLIGHT {
		Some(Unavailable::AtCapacity)
	} else {
		None
	};
	if let Some(refusal) = refusal {
		out.execute(Effect::MigrateActorResponse {
			request,
			result: MigrateActorResult::Unavailable(refusal),
		});
		return;
	}

	let op = state.next_op();
	state.requested = Some(Requested {
		request,
		v2_actor_id,
		step: RequestedStep::ReadingCandidate { op },
	});
	out.arm_timer_after(Timer::Io(op), CANDIDATE_TIMEOUT_MS);
	out.execute(Effect::ReadCandidateRequest { op, v2_actor_id });
}

/// Admits the requested actor V2 described, or answers why not.
fn on_candidate(
	state: &mut State,
	op: OpId,
	result: CandidateResult,
	now: Clocks,
	out: &mut Effects,
) {
	let Some(requested) = state
		.requested
		.take_if(|requested| requested.step == RequestedStep::ReadingCandidate { op })
	else {
		return;
	};
	out.cancel_timer(Timer::Io(op));

	let candidate = match result {
		CandidateResult::Found(candidate) => candidate,
		CandidateResult::NotFound => {
			out.execute(Effect::MigrateActorResponse {
				request: requested.request,
				result: MigrateActorResult::NotFound,
			});
			return;
		}
		CandidateResult::Failed(error) => {
			out.execute(Effect::MigrateActorResponse {
				request: requested.request,
				result: MigrateActorResult::Failed(error),
			});
			return;
		}
	};

	// A page admitted while the read ran may have taken the actor already.
	let selection = match select_requested(&candidate, &state.settings, now.unix_ms) {
		Ok(()) if state.entries.contains_key(&candidate.actor_id) => Err(Reason::InFlight),
		selection => selection,
	};
	match selection {
		Ok(()) => {}
		Err(Reason::InFlight) => {
			on_migrate_actor_request(state, requested.request, requested.v2_actor_id, out);
			return;
		}
		Err(reason) => {
			tracing::info!(
				v2_actor_id = %candidate.actor_id,
				reason = reason.as_str(),
				"refusing an operator request to migrate a V2 actor"
			);
			out.execute(Effect::MigrateActorResponse {
				request: requested.request,
				result: MigrateActorResult::Ineligible(reason),
			});
			return;
		}
	}

	let Some(pass) = state.pass.as_mut() else {
		out.execute(Effect::MigrateActorResponse {
			request: requested.request,
			result: MigrateActorResult::Unavailable(Unavailable::Loading),
		});
		return;
	};
	pass.counts.selected += 1;
	let seq = pass.next_migration_seq;
	pass.next_migration_seq += 1;
	state.pass_dirty = true;

	let datacenter_label = candidate.actor_id.label();
	let entry = new_entry(&candidate, datacenter_label, seq, &state.settings, now);
	let migration_id = entry.record.migration_id;
	let v3_actor_id = v3_actor_id(&candidate, &state.settings);
	tracing::info!(
		v2_actor_id = %candidate.actor_id,
		%v3_actor_id,
		migration_id,
		"admitted a V2 actor an operator requested"
	);
	state.entries.insert(candidate.actor_id, entry);
	state.requested_entries.insert(candidate.actor_id);
	// The operator asked for this actor, so the budget does not hold it back, but its copy still
	// counts against what the listing may admit next.
	let bytes = copy_bytes(&candidate);
	charge_copy(&mut state.copy_budget_unix_ms, &state.settings, bytes, now);
	out.execute(Effect::RecordCopyAdmitted { bytes });
	state.requested = Some(Requested {
		request: requested.request,
		v2_actor_id: requested.v2_actor_id,
		step: RequestedStep::Storing {
			v3_actor_id,
			migration_id,
		},
	});
}

/// Answers the requested actor once a write stored its entry. An entry that already finished was
/// stored before it could run, so it is answered too.
fn answer_stored_request(state: &mut State, out: &mut Effects) {
	let Some(Requested {
		request,
		v2_actor_id,
		step: RequestedStep::Storing {
			v3_actor_id,
			migration_id,
		},
	}) = state.requested.clone()
	else {
		return;
	};
	let stored = state
		.entries
		.get(&v2_actor_id)
		.is_none_or(|entry| !matches!(entry.step, Step::Dirty | Step::Persisting(_)));
	if !stored {
		return;
	}
	state.requested = None;
	out.execute(Effect::MigrateActorResponse {
		request,
		result: MigrateActorResult::Admitted {
			v3_actor_id,
			migration_id,
		},
	});
}

/// Accepts a response for the entry's running effect. Returns false for a stale response.
fn take_running(state: &mut State, v2_actor_id: Id, op: OpId, out: &mut Effects) -> bool {
	let Some(entry) = state.entries.get_mut(&v2_actor_id) else {
		return false;
	};
	if entry.step != Step::Running(op) {
		return false;
	}
	out.cancel_timer(Timer::Io(op));
	entry.step = Step::Ready;
	true
}

fn set_phase(state: &mut State, v2_actor_id: Id, phase: wire::MigrateFromV2Phase) {
	if let Some(entry) = state.entries.get_mut(&v2_actor_id) {
		entry.record.phase = phase;
		entry.step = Step::Dirty;
		entry.failures = 0;
		entry.vanished = 0;
	}
}

fn discard(state: &mut State, v2_actor_id: Id, reason: wire::MigrateFromV2DiscardReason) {
	set_phase(
		state,
		v2_actor_id,
		wire::MigrateFromV2Phase::MigrateFromV2Discarding(wire::MigrateFromV2Discarding { reason }),
	);
}

fn finish(state: &mut State, v2_actor_id: Id, outcome: Outcome, reason: Reason) {
	if let Some(entry) = state.entries.get_mut(&v2_actor_id) {
		entry.finished = Some((outcome, reason));
		entry.step = Step::Dirty;
	}
	if let Some(pass) = state.pass.as_mut() {
		match outcome {
			Outcome::Confirmed => pass.counts.confirmed += 1,
			Outcome::NeedsRecovery => pass.counts.needs_recovery += 1,
			Outcome::Skipped if reason == Reason::DuplicateKey => pass.counts.duplicate_key += 1,
			Outcome::Woke | Outcome::Destroyed | Outcome::Skipped | Outcome::Failed => {
				pass.counts.discarded += 1
			}
		}
		state.pass_dirty = true;
	}
}

/// An activation found no actor at the entry's placement. It is retried, and after
/// [`HOLD_VANISHED_MAX_ATTEMPTS`] such answers within this phase of this tenure, consecutive or
/// not (the count is reset only with the phase and is not persisted), the entry finishes as
/// needing recovery: V2 parked
/// the actor, and nothing at the placement is left to activate, so only an operator can resolve
/// it. Finishing frees its slot and, at the legacy placement, its namespace's backfill, which has
/// nothing there to move.
fn on_activation_vanished(state: &mut State, v2_actor_id: Id, error: &str, out: &mut Effects) {
	let Some(entry) = state.entries.get_mut(&v2_actor_id) else {
		return;
	};
	entry.vanished += 1;
	if entry.vanished < HOLD_VANISHED_MAX_ATTEMPTS {
		retry_later(state, v2_actor_id, error, out);
		return;
	}

	tracing::warn!(
		%v2_actor_id,
		v3_actor_id = ?Id::from_slice(&entry.record.v3_actor_id).ok(),
		migration_id = entry.record.migration_id,
		grouped = entry.record.group.is_some(),
		attempts = entry.vanished,
		%error,
		"V2 parked this actor but its V3 hold is gone from the placement its migration entry names; it is served by neither until an operator resolves it"
	);
	finish(
		state,
		v2_actor_id,
		Outcome::NeedsRecovery,
		Reason::HoldVanished,
	);
}

/// The namespace an entry was admitted for, as its effects carry it: `None` for an entry written
/// by version 1, which did not record it.
fn entry_namespace(record: &wire::MigrateFromV2InFlight) -> Option<String> {
	record
		.namespace
		.as_ref()
		.map(|namespace| namespace.to_string())
}

/// Keeps the entry's phase and reissues its effect after a backoff.
fn retry_later(state: &mut State, v2_actor_id: Id, error: &str, out: &mut Effects) {
	let Some(entry) = state.entries.get_mut(&v2_actor_id) else {
		return;
	};
	entry.failures += 1;
	let phase = snapshot::phase_name(&entry.record.phase);
	tracing::warn!(
		%v2_actor_id,
		phase,
		failures = entry.failures,
		%error,
		"migration step failed; retrying the same request"
	);
	entry.step = Step::Waiting;
	out.arm_timer_after(Timer::Entry(v2_actor_id), backoff_ms(entry.failures));
	out.execute(Effect::RecordEffectFailure {
		phase: phase.to_owned(),
	});
}

fn discard_outcome(entry: &Entry) -> (Outcome, Reason) {
	let wire::MigrateFromV2Phase::MigrateFromV2Discarding(discarding) = &entry.record.phase else {
		return (Outcome::Failed, Reason::None);
	};
	match discarding.reason {
		wire::MigrateFromV2DiscardReason::FenceMoved => (Outcome::Woke, Reason::FenceMoved),
		wire::MigrateFromV2DiscardReason::Destroyed => (Outcome::Destroyed, Reason::Destroyed),
		wire::MigrateFromV2DiscardReason::Ineligible => (Outcome::Skipped, Reason::Ineligible),
		wire::MigrateFromV2DiscardReason::Woke => (Outcome::Woke, Reason::Woke),
		wire::MigrateFromV2DiscardReason::WokeAfterCut => (Outcome::Woke, Reason::WokeAfterCut),
		wire::MigrateFromV2DiscardReason::CopyFailed => (Outcome::Failed, Reason::CopyFailed),
		wire::MigrateFromV2DiscardReason::KeyReservationFailed => {
			(Outcome::Failed, Reason::KeyReservationFailed)
		}
		wire::MigrateFromV2DiscardReason::Failover => (Outcome::Failed, Reason::Failover),
		wire::MigrateFromV2DiscardReason::DuplicateKey => (Outcome::Skipped, Reason::DuplicateKey),
	}
}

fn on_copied(state: &mut State, v2_actor_id: Id, result: CopyResult) {
	match result {
		CopyResult::Copied { .. } => set_phase(
			state,
			v2_actor_id,
			wire::MigrateFromV2Phase::MigrateFromV2Reserving,
		),
		// The begin was refused, so nothing was written to the native actor.
		CopyResult::NativeActor => {
			finish(state, v2_actor_id, Outcome::Skipped, Reason::DuplicateKey)
		}
		CopyResult::PlacementMoved { group } => move_placement(state, v2_actor_id, group),
		CopyResult::FenceMoved => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::FenceMoved,
		),
		CopyResult::Destroyed => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::Destroyed,
		),
		CopyResult::Ineligible => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::Ineligible,
		),
		CopyResult::Failed(error) => {
			tracing::warn!(%v2_actor_id, %error, "migration copy failed; discarding the attempt");
			discard(
				state,
				v2_actor_id,
				wire::MigrateFromV2DiscardReason::CopyFailed,
			);
		}
	}
}

/// The copy of a legacy-placed `Copying` entry found the namespace the entry was admitted for in
/// the volume group `group` before it sent the begin, so nothing was written for the entry. The
/// entry is placed in the group in place, with the id a keyed actor takes there, and stored again
/// before its copy is issued. An operator request still waiting for the entry to be stored
/// answers with the new id.
///
/// Later admissions are placed in the group only when the entry was admitted for the configured
/// target namespace. After a retarget the group belongs to the entry's earlier namespace, and an
/// entry written by version 1 names no namespace, so neither may place the new target's
/// admissions; those keep the target's own standing.
fn move_placement(state: &mut State, v2_actor_id: Id, group: [u8; 16]) {
	let Some(entry) = state.entries.get(&v2_actor_id) else {
		return;
	};
	let admitted_for_target =
		entry.record.namespace.as_deref() == Some(state.settings.target_namespace.as_str());
	if admitted_for_target {
		state.settings.target_group = Some(group);
	}

	let settings = state.settings.clone();
	let Some(entry) = state.entries.get_mut(&v2_actor_id) else {
		return;
	};
	let candidate = &entry.record.candidate;
	let v3_actor_id = placed_v3_actor_id(
		v2_actor_id,
		&*candidate.name,
		candidate.key.as_deref(),
		true,
		&settings,
	);
	tracing::info!(
		%v2_actor_id,
		%v3_actor_id,
		"the target namespace has a volume group now; placing the migration there before its begin"
	);
	entry.record.group = Some(group);
	entry.record.v3_actor_id = v3_actor_id.as_bytes().into();
	entry.placement_moved = true;
	if let Some(Requested {
		v2_actor_id: requested,
		step: RequestedStep::Storing {
			v3_actor_id: answered,
			..
		},
		..
	}) = state.requested.as_mut()
		&& *requested == v2_actor_id
	{
		*answered = v3_actor_id;
	}
	set_phase(
		state,
		v2_actor_id,
		wire::MigrateFromV2Phase::MigrateFromV2Copying,
	);
}

/// Cuts V2 over once the held actor's key is reserved. The actor state machine owns the
/// reservation and sends no notification, so a pending reservation is read again after a
/// backoff; it settles within the actor's bounded reservation attempts.
fn on_reservation(
	state: &mut State,
	v2_actor_id: Id,
	result: ReservationResult,
	now: Clocks,
	out: &mut Effects,
) {
	match result {
		ReservationResult::Reserved => set_phase(
			state,
			v2_actor_id,
			wire::MigrateFromV2Phase::MigrateFromV2CuttingOver(wire::MigrateFromV2CuttingOver {
				cutover_ts: now.unix_ms,
			}),
		),
		ReservationResult::Pending => {
			let Some(entry) = state.entries.get_mut(&v2_actor_id) else {
				return;
			};
			entry.failures += 1;
			tracing::debug!(%v2_actor_id, "key reservation pending; reading it again");
			entry.step = Step::Waiting;
			out.arm_timer_after(Timer::Entry(v2_actor_id), backoff_ms(entry.failures));
		}
		ReservationResult::OwnedElsewhere => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::DuplicateKey,
		),
		// V2 was never cut over, so discarding is safe; a later pass imports the actor again.
		ReservationResult::Exhausted | ReservationResult::NotHeld => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::KeyReservationFailed,
		),
		ReservationResult::Failed(error) => retry_later(state, v2_actor_id, &error, out),
	}
}

fn on_cutover(
	state: &mut State,
	v2_actor_id: Id,
	result: CutoverResult,
	now: Clocks,
	out: &mut Effects,
) {
	let Some(entry) = state.entries.get(&v2_actor_id) else {
		return;
	};
	let wire::MigrateFromV2Phase::MigrateFromV2CuttingOver(cutting) = &entry.record.phase else {
		return;
	};
	let cutover_ts = cutting.cutover_ts;
	match result {
		CutoverResult::Cut => set_phase(
			state,
			v2_actor_id,
			wire::MigrateFromV2Phase::MigrateFromV2Confirming(wire::MigrateFromV2Confirming {
				cutover_ts,
				confirm_at: now
					.unix_ms
					.saturating_add(state.settings.confirm_delay_ms as i64),
			}),
		),
		CutoverResult::Woke => discard(state, v2_actor_id, wire::MigrateFromV2DiscardReason::Woke),
		CutoverResult::Destroyed => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::Destroyed,
		),
		CutoverResult::Ineligible => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::Ineligible,
		),
		CutoverResult::Failed(error) => retry_later(state, v2_actor_id, &error, out),
	}
}

fn on_confirm(state: &mut State, v2_actor_id: Id, result: ConfirmResult, out: &mut Effects) {
	let Some(entry) = state.entries.get(&v2_actor_id) else {
		return;
	};
	let wire::MigrateFromV2Phase::MigrateFromV2Confirming(confirming) = &entry.record.phase else {
		return;
	};
	let cutover_ts = confirming.cutover_ts;
	match result {
		ConfirmResult::Confirmed => set_phase(
			state,
			v2_actor_id,
			wire::MigrateFromV2Phase::MigrateFromV2Activating(wire::MigrateFromV2Activating {
				cutover_ts,
			}),
		),
		ConfirmResult::Woke => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::WokeAfterCut,
		),
		ConfirmResult::Destroyed => discard(
			state,
			v2_actor_id,
			wire::MigrateFromV2DiscardReason::Destroyed,
		),
		// V2 parked the actor under another cutover, so another attempt owns the hand-over. V2
		// answers this the same way on every retry. Nothing is discarded, because the V3 hold may
		// be the only live copy.
		ConfirmResult::CutoverMismatch => {
			tracing::error!(
				%v2_actor_id,
				cutover_ts,
				"V2 parked this actor under a different cutover; it needs an operator to resolve it"
			);
			finish(
				state,
				v2_actor_id,
				Outcome::NeedsRecovery,
				Reason::CutoverMismatch,
			);
		}
		ConfirmResult::Failed(error) => retry_later(state, v2_actor_id, &error, out),
	}
}

fn on_io_timeout(state: &mut State, op: OpId, now: Clocks, out: &mut Effects) {
	// A load, a write and a listing time out like a failure of the same request.
	if state.phase == (Phase::Loading { op }) {
		on_loaded(state, op, LoadResult::Failed("timed out".into()), now, out);
		return;
	}
	if let Persist::Writing(persist) = &state.persist
		&& persist.op == op
	{
		on_written(state, op, WriteResult::Failed("timed out".into()), now, out);
		return;
	}
	if state
		.requested
		.as_ref()
		.is_some_and(|requested| requested.step == RequestedStep::ReadingCandidate { op })
	{
		on_candidate(
			state,
			op,
			CandidateResult::Failed("reading the actor from V2 timed out".into()),
			now,
			out,
		);
		return;
	}
	if let Listing::Requesting {
		op: listing_op,
		datacenter_label,
	} = state.listing
		&& listing_op == op
	{
		on_page(
			state,
			op,
			datacenter_label,
			ListResult::Failed("timed out".into()),
			now,
			out,
		);
		return;
	}

	let Some(v2_actor_id) = state
		.entries
		.iter()
		.find(|(_, entry)| entry.step == Step::Running(op))
		.map(|(id, _)| *id)
	else {
		return;
	};
	let copying = state.entries.get(&v2_actor_id).is_some_and(|entry| {
		matches!(
			entry.record.phase,
			wire::MigrateFromV2Phase::MigrateFromV2Copying
		)
	});
	if let Some(entry) = state.entries.get_mut(&v2_actor_id) {
		entry.step = Step::Ready;
	}
	if copying {
		on_copied(
			state,
			v2_actor_id,
			CopyResult::Failed("copy timed out".into()),
		);
	} else {
		retry_later(state, v2_actor_id, "timed out", out);
	}
}

/// Runs after every event while running: persists changed entries, issues the effects of stored
/// entries, and lists the next page.
fn dispatch(state: &mut State, now: Clocks, out: &mut Effects) {
	persist(state, out);
	issue_entry_effects(state, out);
	list(state, now, out);
}

/// Writes changed entries and a changed pass entry while `persist` is `Idle`. A write in flight
/// or a failed write's backoff holds the changes until it ends.
fn persist(state: &mut State, out: &mut Effects) {
	if state.persist != Persist::Idle {
		return;
	}
	let mut puts = Vec::new();
	let mut deletes = Vec::new();
	for (v2_actor_id, entry) in &state.entries {
		if entry.step != Step::Dirty {
			continue;
		}
		if entry.finished.is_some() {
			deletes.push(*v2_actor_id);
		} else {
			puts.push(*v2_actor_id);
		}
	}
	if puts.is_empty() && deletes.is_empty() && !state.pass_dirty {
		return;
	}

	let op = state.next_op();
	let mut records = Vec::with_capacity(puts.len());
	for v2_actor_id in puts.iter().chain(deletes.iter()) {
		if let Some(entry) = state.entries.get_mut(v2_actor_id) {
			entry.step = Step::Persisting(op);
			if entry.finished.is_none() {
				records.push(entry.record.clone());
			}
		}
	}
	let pass = if state.pass_dirty {
		state.pass.clone()
	} else {
		None
	};
	state.pass_dirty = false;
	state.persist = Persist::Writing(PersistOp {
		op,
		puts,
		deletes: deletes.clone(),
	});
	out.arm_timer_after(Timer::Io(op), IO_TIMEOUT_MS);
	out.execute(Effect::WriteStateRequest {
		op,
		tenure: state.tenure,
		pass,
		puts: records,
		deletes,
	});
}

fn issue_entry_effects(state: &mut State, out: &mut Effects) {
	let horizon_ms = state.settings.wake_horizon_ms as i64;
	let mut issued = Vec::new();
	for (v2_actor_id, entry) in &state.entries {
		if entry.step != Step::Ready || entry.finished.is_some() {
			continue;
		}
		issued.push((*v2_actor_id, entry.record.clone()));
	}

	for (v2_actor_id, record) in issued {
		let op = state.next_op();
		let v3_actor_id = Id::from_slice(&record.v3_actor_id).unwrap_or(v2_actor_id);
		let migration_id = record.migration_id;
		let group = record.group;
		let namespace = entry_namespace(&record);
		let (effect, timeout_ms) = match &record.phase {
			wire::MigrateFromV2Phase::MigrateFromV2Copying => (
				Effect::CopyRequest {
					op,
					v2_actor_id,
					v3_actor_id,
					migration_id,
					candidate: record.candidate.clone(),
					group,
					namespace,
				},
				COPY_TIMEOUT_MS,
			),
			wire::MigrateFromV2Phase::MigrateFromV2Reserving => (
				Effect::ReadReservationRequest {
					op,
					v2_actor_id,
					v3_actor_id,
					migration_id,
					group,
				},
				IO_TIMEOUT_MS,
			),
			wire::MigrateFromV2Phase::MigrateFromV2CuttingOver(cutting) => (
				Effect::CutoverRequest {
					op,
					v2_actor_id,
					generation: record.candidate.generation,
					cutover_ts: cutting.cutover_ts,
				},
				IO_TIMEOUT_MS,
			),
			wire::MigrateFromV2Phase::MigrateFromV2Confirming(confirming) => (
				Effect::ConfirmRequest {
					op,
					v2_actor_id,
					v3_actor_id,
					candidate: record.candidate.clone(),
					cutover_ts: confirming.cutover_ts,
					horizon_ms,
				},
				IO_TIMEOUT_MS,
			),
			wire::MigrateFromV2Phase::MigrateFromV2Activating(_) => (
				Effect::ActivateRequest {
					op,
					v2_actor_id,
					v3_actor_id,
					migration_id,
					candidate: record.candidate.clone(),
					group,
				},
				IO_TIMEOUT_MS,
			),
			wire::MigrateFromV2Phase::MigrateFromV2Discarding(_) => (
				Effect::DiscardRequest {
					op,
					v2_actor_id,
					v3_actor_id,
					migration_id,
					group,
				},
				IO_TIMEOUT_MS,
			),
		};
		if let Some(entry) = state.entries.get_mut(&v2_actor_id) {
			entry.step = Step::Running(op);
		}
		out.arm_timer_after(Timer::Io(op), timeout_ms);
		out.execute(effect);
	}
}

fn list(state: &mut State, now: Clocks, out: &mut Effects) {
	if state.listing != Listing::Idle || !state.settings.enabled {
		return;
	}
	let Some(pass) = state.pass.as_mut() else {
		return;
	};

	// A configured datacenter missing from the pass joins it with a fresh cursor.
	for label in &state.settings.datacenter_labels {
		if !pass
			.cursors
			.iter()
			.any(|cursor| cursor.datacenter_label == *label)
		{
			pass.cursors.push(wire::MigrateFromV2Cursor {
				datacenter_label: *label,
				after: None,
				done: false,
			});
			state.pass_dirty = true;
		}
	}

	// A cursor whose datacenter left the configuration has no V2 API to list, so it is skipped.
	// It keeps its position in case the datacenter is configured again during this pass.
	let Some(cursor) = pass
		.cursors
		.iter()
		.find(|cursor| {
			!cursor.done
				&& state
					.settings
					.datacenter_labels
					.contains(&cursor.datacenter_label)
		})
		.cloned()
	else {
		pass.ended_at = Some(now.unix_ms);
		state.pass_dirty = true;
		state.listing = Listing::PassDone;
		out.execute(Effect::ReportPassEnd { pass: pass.clone() });
		out.arm_timer_after(Timer::Pass, state.settings.pass_interval_ms);
		return;
	};

	if free_slots(state) == 0 {
		return;
	}
	// Wait for both the actor pacing and the copy budget, so a listed page can admit something.
	let admit_at = state
		.next_admit_unix_ms
		.max(state.copy_budget_unix_ms.saturating_sub(COPY_BURST_MS));
	if now.unix_ms < admit_at {
		state.listing = Listing::Backoff;
		out.arm_timer_at(Timer::Listing, boot_at(now, admit_at));
		return;
	}

	let op = state.next_op();
	state.listing = Listing::Requesting {
		op,
		datacenter_label: cursor.datacenter_label,
	};
	out.arm_timer_after(Timer::Io(op), IO_TIMEOUT_MS);
	out.execute(Effect::ListCandidatesRequest {
		op,
		datacenter_label: cursor.datacenter_label,
		after: cursor.after.map(|after| after.to_vec()),
		limit: PAGE_LIMIT,
	});
}
