//! Detection and repair of known workflow history defects.
//!
//! This works on raw history rows because the defects it handles make the workflow's history
//! undecodable, so nothing above the key layer can see them.
//!
//! Automatic variants have exactly one correct remedy. Each validates the structure it expects
//! before writing, revalidates it inside the mutating transaction, and refuses on any mismatch
//! rather than guessing. Detect-only variants recognize a symptom whose remedy depends on which
//! side of an inconsistency is authoritative; they report and stop.

use std::{cmp::Ordering, collections::BTreeMap, str::FromStr};

use anyhow::{Context, Result, bail, ensure};
use futures_util::TryStreamExt;
use rivet_util::Id;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{
		FormalChunkedKey, FormalKey,
		IsolationLevel::*,
		Subspace,
		keys::{ACTOR, CREATE_TS, DATA, PEGBOARD, RIVET},
	},
};

use super::{DatabaseKv, keys, update_metric};
use crate::{
	db::Database,
	db::debug::{
		RepairCheck, RepairInspection, RepairMode, RepairOutcome, RepairState, RepairVariant,
		RepairVerification, RepairVerifyState,
	},
	history::{
		event::{EventType, SleepState},
		location::{Coordinate, Location},
	},
};

// Owned by this module so the tests can reach the insertion coordinate and error parsing, which are
// private to a private module and cannot be exercised from an integration test.
#[cfg(test)]
#[path = "../../../tests/modules/repair.rs"]
mod tests;

/// The automatic repairs are specific to this workflow's history shape and are refused for anything
/// else. The detect-only symptoms are inconsistencies in gasoline's own primitives and apply to any
/// workflow, so they do not check this.
const ACTOR2_WORKFLOW_NAME: &str = "pegboard_actor2";
const DEALLOCATE_ACTIVITY: &str = "deallocate";
/// Signal that tears down whatever state replay resumes in. Unlike every other actor signal its
/// handler is not gated on the current transition, so it lands in all of them.
const ACTOR2_LOST_SIGNAL: &str = "pegboard_actor2_lost";
/// Dropping this signal is unrecoverable. Its handler only marks the actor as destroying, so a
/// discarded destroy leaves the actor alive with nothing to retry it.
const ACTOR2_DESTROY_SIGNAL: &str = "pegboard_actor2_destroy";
const SET_ERROR_ACTIVITY: &str = "set_error";
const PROPOSE_ACTIVITY: &str = "propose";
const RESERVE_ACTOR_KEY_ACTIVITY: &str = "reserve_actor_key";
const INSERT_STATE_ACTIVITY: &str = "insert_state_and_db";
const LOOKUP_KEY_ACTIVITY: &str = "lookup_key_optimistic";

/// The `propose` outputs the workflow itself handles. Anything else reaches its `unreachable` arm,
/// which is the defect `ProposeConsensusFailed` repairs.
const PROPOSE_HANDLED_OUTPUTS: [&str; 2] = ["\"Committed\"", "ExpectedValueDoesNotMatch"];

/// `pegboard_actor2::SetErrorInput { error: ActorError::EnvoyNoResponse { envoy_key: None } }` as
/// gasoline persists activity input.
///
/// The real stop reason is not recoverable from a diverged history, so the inserted activity
/// records the envoy failure that produces this divergence in the first place. `set_error` never
/// overwrites an error that is already an envoy failure, so if the workflow recorded a more
/// specific cause the inserted activity is a no-op.
const SET_ERROR_INPUT: &str = r#"{"error":{"envoy_no_response":{"envoy_key":null}}}"#;

/// Refuse pathological histories instead of blowing the transaction budget on them.
const MAX_SCANNED_KEYS: usize = 50_000;

/// The raw field data written at one history location, before gasoline decodes it into an `Event`.
/// The repairs work at this level precisely because these defects make decoding fail.
#[derive(Default)]
struct RawEvent {
	event_type: Option<EventType>,
	name: Option<String>,
	version: Option<usize>,
	create_ts: Option<i64>,
	deadline_ts: Option<i64>,
	sleep_state: Option<SleepState>,
	/// Loop events only. The iteration the loop resumes at.
	iteration: Option<usize>,
	/// Signals events only. Names of the signals this event consumed.
	signal_names: Vec<String>,
	input_chunks: Vec<Vec<u8>>,
	output_chunks: Vec<Vec<u8>>,
	has_output: bool,
	/// Name of every field present, including fields this module does not decode. Used to prove a
	/// row holds nothing but the field a repair is about to clear.
	fields: Vec<&'static str>,
	/// Every raw key at this location, so a repair clears exactly what it validated.
	keys: Vec<Vec<u8>>,
}

impl RawEvent {
	fn input(&self) -> String {
		String::from_utf8_lossy(&self.input_chunks.concat()).into_owned()
	}

	fn output(&self) -> String {
		String::from_utf8_lossy(&self.output_chunks.concat()).into_owned()
	}

	fn is_activity(&self, name: &str) -> bool {
		self.event_type == Some(EventType::Activity) && self.name.as_deref() == Some(name)
	}

	/// An active row holding nothing but `sleep_state`. The missing `event_type` is what makes the
	/// workflow's history reader fail with `missing event data: event_type`.
	fn is_sleep_orphan(&self) -> bool {
		self.sleep_state.is_some() && self.fields == ["sleep_state"]
	}

	/// A Sleep event with every field loop compaction should have carried into forgotten history.
	fn is_complete_sleep(&self) -> bool {
		self.event_type == Some(EventType::Sleep)
			&& self.version.is_some()
			&& self.create_ts.is_some()
			&& self.deadline_ts.is_some()
			&& self.sleep_state.is_some()
	}

	fn describe(&self) -> String {
		match (self.event_type, &self.name) {
			(Some(event_type), Some(name)) => format!("{event_type} {name:?}"),
			(Some(event_type), None) => event_type.to_string(),
			(None, _) => format!("no event type, fields: {}", self.fields.join(", ")),
		}
	}
}

/// Workflow level state every repair validates against.
struct WorkflowMeta {
	name: Option<String>,
	error: Option<String>,
	/// The workflow's durable state. Unlike loop state this is committed as each activity finishes,
	/// so it describes what actually happened rather than what replay would reproduce.
	state: Option<String>,
	has_lease: bool,
	has_worker: bool,
	has_wake_condition: bool,
	is_silenced: bool,
	is_complete: bool,
}

/// What a repair will write, derived during inspection and reused inside the mutating transaction.
enum RepairPlan {
	DeallocateSetError {
		insertion_location: Location,
		/// Copied from the `deallocate` the inserted event runs before. Gasoline compares activity
		/// versions during replay, so a mismatch would diverge again.
		version: usize,
	},
	OrphanedSleepState {
		location: Location,
		orphan_keys: Vec<Vec<u8>>,
		orphan_sleep_state: SleepState,
		workflow_name: String,
	},
	AdvanceLoopIteration {
		loop_location: Location,
		/// Highest recorded branch coordinate under the loop. Replay resumes at `+ 1`, which is
		/// past every recorded event.
		new_iteration: usize,
		previous_iteration: usize,
		/// Generation the follow up `Lost` signal targets. `Main::Lost` ignores signals for any
		/// other generation.
		generation: u64,
	},
	/// Clears the last recorded event so the activity runs again on the next wake. Only valid for
	/// the trailing event: the replay cursor is index based, so clearing anything earlier shifts
	/// every event after it.
	ClearTrailingEvent {
		location: Location,
		/// Every raw key the inspection validated at that location. Cleared exactly rather than by
		/// range, so a row written since the scan is never destroyed silently.
		keys: Vec<Vec<u8>>,
		/// What was cleared, for the mutation log.
		description: String,
	},
	/// Writes the workflow state the init activity recorded and clears the trailing activity that
	/// failed reading it.
	WriteInitState {
		location: Location,
		keys: Vec<Vec<u8>>,
		/// `pegboard_actor2::State` rebuilt from the recorded `insert_state_and_db` input.
		state: String,
	},
}

/// Work a repair still has to do once its transaction has committed.
#[derive(Default)]
struct PostCommit {
	/// The repair armed a wake itself, so the caller must not wake the workflow again.
	wake_armed: bool,
	/// Send a `Lost` signal for this generation, so the workflow tears down the state it resumes
	/// in rather than continuing from it.
	lost_generation: Option<u64>,
	/// Loop branches the advance left behind in active history, to move into forgotten one
	/// transaction at a time before the workflow is woken.
	drain_branches: Vec<Location>,
}

/// Accumulates the checks a repair ran so the operator sees exactly what was validated.
struct InspectionBuilder {
	variant: RepairVariant,
	workflow_id: Id,
	mode: RepairMode,
	workflow_name: Option<String>,
	workflow_error: Option<String>,
	location: Option<Location>,
	has_lease: bool,
	has_worker: bool,
	has_wake_condition: bool,
	checks: Vec<RepairCheck>,
}

impl InspectionBuilder {
	fn new(variant: RepairVariant, workflow_id: Id) -> Self {
		InspectionBuilder {
			variant,
			workflow_id,
			mode: variant.mode(),
			workflow_name: None,
			workflow_error: None,
			location: None,
			has_lease: false,
			has_worker: false,
			has_wake_condition: false,
			checks: Vec::new(),
		}
	}

	fn apply_meta(&mut self, meta: &WorkflowMeta) {
		self.workflow_name = meta.name.clone();
		self.workflow_error = meta.error.clone();
		self.has_lease = meta.has_lease;
		self.has_worker = meta.has_worker;
		self.has_wake_condition = meta.has_wake_condition;
	}

	fn pass(&mut self, name: impl Into<String>, detail: impl Into<String>) {
		self.checks.push(RepairCheck::pass(name, detail));
	}

	/// Reports the defect but refuses to write the remedy, for a workflow this repair recognizes and
	/// cannot safely fix on its own. The check explains what a human has to settle.
	fn manual(&mut self, name: impl Into<String>, detail: impl Into<String>) -> RepairInspection {
		self.mode = RepairMode::ManualOnly;
		self.checks.push(RepairCheck::pass(name, detail));
		self.finish(RepairState::Ready)
	}

	fn refuse(&mut self, name: impl Into<String>, detail: impl Into<String>) -> RepairInspection {
		self.checks.push(RepairCheck::fail(name, detail));
		self.finish(RepairState::NotApplicable)
	}

	fn finish(&mut self, state: RepairState) -> RepairInspection {
		RepairInspection {
			variant: self.variant,
			workflow_id: self.workflow_id,
			workflow_name: self.workflow_name.take(),
			workflow_error: self.workflow_error.take(),
			state,
			mode: self.mode,
			location: self.location.take(),
			has_lease: self.has_lease,
			has_worker: self.has_worker,
			has_wake_condition: self.has_wake_condition,
			checks: std::mem::take(&mut self.checks),
		}
	}
}

impl DatabaseKv {
	pub(crate) async fn inspect_workflow_repair_inner(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Option<Location>,
	) -> Result<RepairInspection> {
		self.pools
			.udb()?
			.txn("gas_debug_inspect_workflow_repair", |tx| {
				let location = location.clone();

				async move {
					self.inspect(&tx, workflow_id, variant, location)
						.await
						.map(|(inspection, _)| inspection)
				}
			})
			.await
	}

	pub(crate) async fn repair_workflow_inner(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Option<Location>,
	) -> Result<RepairOutcome> {
		let outcome = self
			.pools
			.udb()?
			.txn("gas_debug_repair_workflow", |tx| {
				let location = location.clone();

				async move {
					// Revalidate everything inside the mutating transaction. The reads this
					// performs become the transaction's read conflict ranges, so any concurrent
					// change to the history, the lease, or the workflow state fails the repair
					// instead of racing it.
					let (inspection, plan) =
						self.inspect(&tx, workflow_id, variant, location).await?;

					// A detect-only symptom recognizes a defect but cannot settle which side of the
					// inconsistency is authoritative, so there is no remedy to apply. An inspection
					// downgrades an otherwise automatic repair the same way when this specific
					// workflow fails a precondition the remedy depends on.
					ensure!(
						inspection.mode == RepairMode::Automatic,
						"{variant} has no automatic repair for {workflow_id}, a human has to read the history and repair it by hand",
					);

					match inspection.state {
						RepairState::AlreadyApplied => {
							return Ok((
								RepairOutcome {
									inspection,
									changed: false,
									wake_armed: false,
									mutations: Vec::new(),
								},
								None,
								Vec::new(),
							));
						}
						RepairState::NotApplicable => {
							let reason = inspection
								.checks
								.iter()
								.find(|check| !check.passed)
								.map(|check| format!("{}: {}", check.name, check.detail))
								.unwrap_or_else(|| {
									"workflow does not have this defect".to_string()
								});

							bail!("refusing to repair {workflow_id}, {reason}");
						}
						RepairState::Ready => {}
					}

					let plan = plan.context("ready inspection must produce a plan")?;
					let (post_commit, mutations) = self.apply_plan(&tx, workflow_id, plan).await?;

					Ok((
						RepairOutcome {
							inspection,
							changed: true,
							wake_armed: post_commit.wake_armed,
							mutations,
						},
						post_commit.lost_generation,
						post_commit.drain_branches,
					))
				}
			})
			.await?;

		let (mut outcome, lost_generation, drain_branches) = outcome;

		// Runs before the wake so the workflow never sees the oversized range.
		if !drain_branches.is_empty() {
			let mut moved = 0;
			for branch in &drain_branches {
				moved += self.move_branch_to_forgotten(workflow_id, branch).await?;
			}

			outcome.mutations.push(format!(
				"moved {} row(s) across {} skipped branch(es) into forgotten history",
				moved,
				drain_branches.len(),
			));
		}

		// Sent after the repair commits rather than written into it, so the signal goes through the
		// normal publish path and its indexes and bumps stay correct. `Main::Lost` is not gated on
		// the workflow's current transition, so it lands whatever state replay resumes in.
		if let Some(generation) = lost_generation {
			let body = serde_json::value::RawValue::from_string(format!(
				r#"{{"generation":{generation},"reason":"envoy_no_response"}}"#
			))
			.context("failed to build lost signal body")?;

			self.publish_signal(
				Id::new_v1(self.config.dc_label()),
				workflow_id,
				Id::new_v1(self.config.dc_label()),
				ACTOR2_LOST_SIGNAL,
				&body,
			)
			.await?;

			outcome.mutations.push(format!(
				"sent {ACTOR2_LOST_SIGNAL} for generation {generation}"
			));
		}

		// The repair released a stranded lease and armed an immediate wake, so let the workers know
		// there is work rather than waiting for the next poll.
		if outcome.wake_armed {
			self.bump(crate::db::BumpSubSubject::Worker);
		}

		Ok(outcome)
	}

	pub(crate) async fn verify_workflow_repair_inner(
		&self,
		workflow_id: Id,
		variant: RepairVariant,
		location: Location,
	) -> Result<RepairVerification> {
		self.pools
			.udb()?
			.txn("gas_debug_verify_workflow_repair", |tx| {
				let location = location.clone();

				async move {
					match variant {
						RepairVariant::DeallocateSetError => {
							self.verify_deallocate_set_error(&tx, workflow_id, location)
								.await
						}
						RepairVariant::OrphanedSleepState => {
							self.verify_orphaned_sleep_state(&tx, workflow_id, location)
								.await
						}
						RepairVariant::LoopIterationMismatch => {
							self.verify_loop_iteration_mismatch(&tx, workflow_id, location)
								.await
						}
						RepairVariant::ProposeConsensusFailed => {
							self.verify_propose_consensus_failed(&tx, workflow_id, location)
								.await
						}
						RepairVariant::MissingInitState => {
							self.verify_missing_init_state(&tx, workflow_id, location)
								.await
						}
						// Nothing was applied, so there is nothing to verify.
						RepairVariant::SleepStateMismatch
						| RepairVariant::DuplicateIterationHistory
						| RepairVariant::IterationTimestampInversion => {
							bail!("{variant} is a detect-only symptom and has no repair to verify")
						}
					}
				}
			})
			.await
	}

	async fn inspect(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		variant: RepairVariant,
		location: Option<Location>,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		match variant {
			RepairVariant::DeallocateSetError => {
				self.inspect_deallocate_set_error(tx, workflow_id).await
			}
			RepairVariant::OrphanedSleepState => {
				self.inspect_orphaned_sleep_state(tx, workflow_id, location)
					.await
			}
			RepairVariant::SleepStateMismatch => {
				self.inspect_sleep_state_mismatch(tx, workflow_id, location)
					.await
			}
			RepairVariant::DuplicateIterationHistory => {
				self.inspect_duplicate_iteration_history(tx, workflow_id, location)
					.await
			}
			RepairVariant::LoopIterationMismatch => {
				self.inspect_loop_iteration_mismatch(tx, workflow_id).await
			}
			RepairVariant::IterationTimestampInversion => {
				self.inspect_iteration_timestamp_inversion(tx, workflow_id)
					.await
			}
			RepairVariant::ProposeConsensusFailed => {
				self.inspect_propose_consensus_failed(tx, workflow_id).await
			}
			RepairVariant::MissingInitState => {
				self.inspect_missing_init_state(tx, workflow_id).await
			}
		}
	}

	/// Detects iteration branches inside one loop whose creation order disagrees with their
	/// coordinate order.
	///
	/// A single run writes iterations in ascending coordinate order, so an inversion means more
	/// than one run wrote into this loop. Only whole iteration branches are compared, because
	/// gasoline legitimately inserts events at earlier coordinates *within* a branch when a
	/// workflow version adds a step.
	async fn inspect_iteration_timestamp_inversion(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder =
			InspectionBuilder::new(RepairVariant::IterationTimestampInversion, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		if let Some(refusal) = refuse_if_missing(&mut builder, &meta) {
			return Ok((refusal, None));
		}

		let active = self.scan_active_history(tx, workflow_id).await?;

		let loops = active
			.iter()
			.filter(|(_, event)| event.event_type == Some(EventType::Loop))
			.map(|(location, _)| location.clone())
			.collect::<Vec<_>>();

		for loop_location in loops {
			// Branches in coordinate order, which is the order a single run writes them in.
			let branches = active
				.iter()
				.filter(|(location, event)| {
					location.root() == loop_location && event.event_type == Some(EventType::Branch)
				})
				.filter_map(|(location, event)| Some((location.clone(), event.create_ts?)))
				.collect::<Vec<_>>();

			let inversions = branches
				.windows(2)
				.filter(|pair| pair[0].1 > pair[1].1)
				.map(|pair| {
					format!(
						"{} was created after {} ({} vs {})",
						pair[0].0, pair[1].0, pair[0].1, pair[1].1
					)
				})
				.collect::<Vec<_>>();

			if inversions.is_empty() {
				continue;
			}

			builder.location = Some(loop_location.clone());
			builder.pass(
				"iteration order",
				format!(
					"{loop_location} has {} branch(es) created out of coordinate order, so more than one run wrote into this loop: {}",
					inversions.len(),
					inversions.join("; "),
				),
			);

			return Ok((builder.finish(RepairState::Ready), None));
		}

		Ok((
			builder.refuse(
				"iteration order",
				"every loop's iteration branches were created in coordinate order",
			),
			None,
		))
	}

	/// Detects a `listen_n_until` Sleep whose `sleep_state` disagrees with the Signals event
	/// recorded after it.
	///
	/// `listen_n_until` writes the Sleep at `{loop, iteration, 1}` and, when signals arrive, the
	/// signal pull transaction writes a Signals event at `{loop, iteration, 2}` and sets
	/// `sleep_state` to `Interrupted`. A Signals event can therefore only follow that Sleep if the
	/// sleep was interrupted, so anything else means `sleep_state` is wrong and replay will take
	/// the timed-out branch and diverge against the recorded Signals event.
	async fn inspect_sleep_state_mismatch(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Option<Location>,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder = InspectionBuilder::new(RepairVariant::SleepStateMismatch, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		if let Some(refusal) = refuse_if_missing(&mut builder, &meta) {
			return Ok((refusal, None));
		}

		let active = self.scan_active_history(tx, workflow_id).await?;

		let mut mismatches = active
			.iter()
			.filter(|(location, event)| {
				event.event_type == Some(EventType::Sleep)
					&& location.tail() == Some(&Coordinate::simple(1))
			})
			.filter_map(|(sleep_location, sleep)| {
				// Whatever `listen_n_until` recorded next lands directly after the Sleep.
				let next_location = sleep_location.root().join(Coordinate::simple(2));
				let next = active.get(&next_location)?;
				let next_is_signals = next.event_type == Some(EventType::Signals);
				let interrupted = sleep.sleep_state == Some(SleepState::Interrupted);

				// `Interrupted` and a following Signals event are the same fact recorded twice, so
				// they always agree. Either direction of disagreement means `sleep_state` is wrong
				// and replay takes the branch history did not record.
				(next_is_signals != interrupted).then(|| {
					(
						sleep_location.clone(),
						sleep.sleep_state,
						next.describe(),
						next_is_signals,
					)
				})
			})
			.collect::<Vec<_>>();

		if let Some(hint) = &location {
			mismatches.retain(|(candidate, ..)| candidate == hint);
		}

		let Some((sleep_location, sleep_state, next, next_is_signals)) =
			mismatches.first().cloned()
		else {
			return Ok((
				builder.refuse(
					"sleep state",
					"no sleep event in active history disagrees with the event recorded after it",
				),
				None,
			));
		};
		builder.location = Some(sleep_location.clone());

		let next_location = sleep_location.root().join(Coordinate::simple(2));
		let state = sleep_state
			.map(|state| state.to_string())
			.unwrap_or_else(|| "<missing>".to_string());
		builder.pass(
			"sleep state",
			if next_is_signals {
				format!(
					"{sleep_location} has state {state} but {next_location} recorded {next}, which only happens when the sleep is interrupted"
				)
			} else {
				format!(
					"{sleep_location} has state {state} but {next_location} recorded {next} rather than signals, so the sleep was not interrupted"
				)
			},
		);

		if mismatches.len() > 1 {
			builder.pass(
				"other occurrences",
				mismatches
					.iter()
					.skip(1)
					.map(|(location, ..)| location.to_string())
					.collect::<Vec<_>>()
					.join(", "),
			);
		}

		Ok((builder.finish(RepairState::Ready), None))
	}

	/// Detects a loop iteration coordinate that holds events in both active and forgotten history,
	/// which means the iteration was recorded twice.
	///
	/// Compaction moves an iteration's events out of active, so an iteration should live in exactly
	/// one subspace. Both means a replay re-ran an iteration whose events had already been
	/// compacted, because `Cursor::compare_loop_branch` reads active history only and cannot see
	/// them.
	async fn inspect_duplicate_iteration_history(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Option<Location>,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder =
			InspectionBuilder::new(RepairVariant::DuplicateIterationHistory, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		if let Some(refusal) = refuse_if_missing(&mut builder, &meta) {
			return Ok((refusal, None));
		}

		let active = self.scan_active_history(tx, workflow_id).await?;

		let iterations = active
			.iter()
			.filter(|(candidate, event)| {
				event.event_type == Some(EventType::Branch)
					&& active
						.get(&candidate.root())
						.and_then(|parent| parent.event_type)
						== Some(EventType::Loop)
			})
			.map(|(candidate, _)| candidate.clone())
			.filter(|candidate| location.as_ref().is_none_or(|hint| candidate == hint))
			.collect::<Vec<_>>();

		for candidate in iterations {
			let iteration_coord = candidate.tail().context("empty branch location")?.clone();
			let forgotten = self
				.scan_raw_history(
					tx,
					&self
						.subspace
						.subspace(&keys::history::EventHistorySubspaceKey::new(
							workflow_id,
							candidate.root(),
							iteration_coord.head(),
							true,
						)),
				)
				.await?;

			let forgotten_events = describe_children(&forgotten, &candidate);
			if forgotten_events.is_empty() {
				continue;
			}
			let active_events = describe_children(&active, &candidate);

			builder.location = Some(candidate.clone());
			builder.pass(
				"duplicate iteration",
				format!("{candidate} holds events in both active and forgotten history"),
			);
			builder.pass(
				"active",
				if active_events.is_empty() {
					"no events, so replay re-runs this iteration against an empty branch"
						.to_string()
				} else {
					format!(
						"{} event(s): {}",
						active_events.len(),
						active_events.join(", ")
					)
				},
			);
			builder.pass(
				"forgotten",
				format!(
					"{} event(s): {}",
					forgotten_events.len(),
					forgotten_events.join(", ")
				),
			);

			// `loope` resumes at the loop event's `iteration` and replays the branch at coordinate
			// `iteration + 1`. Whether this iteration is the one replay reads decides whether it is
			// the blocking failure or residue left behind by it.
			let resume_coord = active
				.get(&candidate.root())
				.and_then(|parent| parent.iteration)
				.map(|iteration| iteration + 1);
			builder.pass(
				"replay position",
				match resume_coord {
					Some(resume) if resume == iteration_coord.head() => {
						"the loop resumes at this branch, so replay reads it directly".to_string()
					}
					Some(resume) => format!(
						"the loop resumes at coordinate {resume}, so replay never reads this branch and it is residue rather than the blocking failure"
					),
					None => "the parent loop event has no iteration counter".to_string(),
				},
			);

			return Ok((builder.finish(RepairState::Ready), None));
		}

		Ok((
			builder.refuse(
				"duplicate iteration",
				"no loop iteration has events in both active and forgotten history",
			),
			None,
		))
	}

	/// Detects replay repeatedly failing inside the loop's current iteration, which means the loop
	/// event's persisted state and the events already recorded for that iteration describe
	/// different runs.
	///
	/// `loope` resumes at the loop event's `iteration` and replays the branch at coordinate
	/// `iteration + 1`, so a failure anchored at or under that coordinate is the loop state and the
	/// recorded iteration disagreeing. Both `latent history found` (the replayed path consumed
	/// fewer events than were recorded) and `history diverged` (the replayed path branched
	/// differently) surface this way.
	async fn inspect_loop_iteration_mismatch(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder = InspectionBuilder::new(RepairVariant::LoopIterationMismatch, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		// Skipping recorded events is only safe for a workflow that self repairs.
		let Some(workflow_name) = meta.name.clone() else {
			return Ok((
				builder.refuse("workflow exists", "workflow not found"),
				None,
			));
		};
		if workflow_name != ACTOR2_WORKFLOW_NAME {
			return Ok((
				builder.refuse(
					"workflow name",
					format!(
						"is {workflow_name:?}, this repair only handles {ACTOR2_WORKFLOW_NAME:?}"
					),
				),
				None,
			));
		}
		builder.pass("workflow name", format!("{workflow_name:?}"));

		if let Some(reason) = live_workflow_refusal(&meta) {
			return Ok((builder.refuse("workflow state", reason), None));
		}
		builder.pass(
			"workflow state",
			"dead, not leased, no pending wake condition",
		);

		let error = meta.error.as_deref().unwrap_or_default();

		let Some(error_location) = parse_error_location(error) else {
			return Ok((
				builder.refuse(
					"workflow error",
					format!("is {error:?}, which does not name a history location"),
				),
				None,
			));
		};

		let active = self.scan_active_history(tx, workflow_id).await?;

		// Find the loop whose resume coordinate contains the location replay failed at.
		let Some((loop_location, iteration, loop_state)) = active
			.iter()
			.filter_map(|(location, event)| {
				(event.event_type == Some(EventType::Loop))
					.then(|| Some((location.clone(), event.iteration?, event.input())))
					.flatten()
			})
			.find(|(loop_location, iteration, _)| {
				// Replay resumes at `iteration + 1` and walks forward through the recorded
				// iterations, so the failure lands at or after that coordinate rather than exactly
				// on it.
				error_location.len() > loop_location.len()
					&& error_location.starts_with(&loop_location[..])
					&& error_location[loop_location.len()].head() >= iteration + 1
			})
		else {
			return Ok((
				builder.refuse(
					"loop iteration",
					format!(
						"{error_location} is not at or under any loop's resume coordinate, so replay is not stuck on a loop iteration"
					),
				),
				None,
			));
		};

		let resume = loop_location.join(Coordinate::simple(iteration + 1));
		// The loop event is what the repair writes and what verification re-reads.
		builder.location = Some(loop_location.clone());

		builder.pass(
			"loop state",
			format!(
				"{loop_location} resumes at iteration {iteration} (branch {resume}) with state {loop_state}"
			),
		);
		builder.pass(
			"replay failure",
			format!("{error_location} is at or after the resume branch: {error}"),
		);

		// The branch replay actually diverged in, which is at or after the resume branch.
		let failing_branch = loop_location.join(error_location[loop_location.len()].clone());
		let recorded = active
			.iter()
			.filter(|(location, event)| {
				location.root() == failing_branch && event.event_type.is_some()
			})
			.map(|(location, event)| format!("{location} {}", event.describe()))
			.collect::<Vec<_>>();

		builder.pass(
			"recorded events",
			format!(
				"{} in {failing_branch}: {}",
				recorded.len(),
				recorded.join(", "),
			),
		);

		// Resume past every recorded branch so no history is left to disagree with. Forgotten
		// branches count too, so a later compaction cannot collide with one.
		let forgotten = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::EventHistorySubspaceKey::entire(
						workflow_id,
						loop_location.clone(),
						true,
					)),
			)
			.await?;
		let highest = active
			.keys()
			.chain(forgotten.keys())
			.filter(|location| location.root() == loop_location)
			.filter_map(|location| location.tail().map(Coordinate::head))
			.max()
			.context("loop has no recorded branches")?;

		if highest <= iteration {
			return Ok((
				builder.refuse(
					"advance target",
					format!(
						"the highest recorded branch coordinate is {highest}, which the loop already passed at iteration {iteration}"
					),
				),
				None,
			));
		}

		let skipped = (iteration + 1..=highest)
			.map(|coord| {
				let branch = loop_location.join(Coordinate::simple(coord));
				let events = describe_children(&active, &branch);

				format!(
					"{branch} ({})",
					if events.is_empty() {
						"no active events".to_string()
					} else {
						events.join(", ")
					}
				)
			})
			.collect::<Vec<_>>();

		builder.pass(
			"advance target",
			format!(
				"iteration {iteration} to {highest}, so replay resumes at {} which has no recorded history",
				loop_location.join(Coordinate::simple(highest + 1)),
			),
		);
		builder.pass(
			"skipped iterations",
			format!(
				"{} branch(es) will never be replayed: {}",
				skipped.len(),
				skipped.join(" | "),
			),
		);

		// The loop state is what replay resumes with, but the workflow's durable state is committed
		// per activity and so already reflects the discarded branches. If the durable state has an
		// envoy the loop state does not, `handle_stopped` will not send a stop command, and
		// `deallocate` removes the envoy's actor row that draining scans, so the actor is left
		// running with nothing able to reach it.
		let state_envoy_key = meta
			.state
			.as_deref()
			.and_then(json_string_field("envoy_key"));
		if !transition_has_envoy(&loop_state)
			&& let Some(envoy_key) = &state_envoy_key
		{
			return Ok((
				builder.refuse(
					"placement",
					format!(
						"the loop state resumes with no envoy but the workflow state is still placed on envoy {envoy_key}. Advancing would deallocate it without telling that envoy to stop the actor"
					),
				),
				None,
			));
		}
		builder.pass(
			"placement",
			match &state_envoy_key {
				Some(envoy_key) => format!(
					"the loop state and the workflow state agree the actor is on envoy {envoy_key}"
				),
				None => "the workflow state has no envoy".to_string(),
			},
		);

		// A discarded signals event was already acked, so its handler never runs. Every other signal
		// this workflow reacts to is recoverable by reconciling, but a dropped destroy means the
		// actor is never torn down and nothing tries again.
		let discarded_signals =
			discarded_signal_names(&active, &loop_location, iteration + 1, highest);
		if discarded_signals
			.iter()
			.any(|name| name == ACTOR2_DESTROY_SIGNAL)
		{
			return Ok((
				builder.refuse(
					"discarded signals",
					format!(
						"a discarded branch consumed {ACTOR2_DESTROY_SIGNAL}, which was already acked. Advancing would drop the destroy and leave the actor alive forever"
					),
				),
				None,
			));
		}
		builder.pass(
			"discarded signals",
			if discarded_signals.is_empty() {
				"none".to_string()
			} else {
				format!(
					"{} already acked and will not be handled again: {}",
					discarded_signals.len(),
					discarded_signals.join(", "),
				)
			},
		);

		// The workflow resumes with the loop state as it stands, which predates everything being
		// discarded. `Lost` is sent afterwards to tear that state down, but it only ever applies to
		// the generation the loop state names. A discarded branch that allocated a newer generation
		// would leave it running with no workflow able to address or stop it.
		let Some(generation) = json_generation(&loop_state) else {
			return Ok((
				builder.refuse(
					"loop generation",
					format!("{loop_location} state has no generation: {loop_state}"),
				),
				None,
			));
		};
		let discarded_generation =
			max_referenced_generation(&active, &loop_location, iteration + 1, highest);

		if let Some(discarded) = discarded_generation
			&& discarded > generation
		{
			return Ok((
				builder.refuse(
					"discarded generation",
					format!(
						"a discarded branch references generation {discarded}, newer than the loop state's {generation}. Advancing would leave that generation running with no workflow able to stop it"
					),
				),
				None,
			));
		}

		builder.pass(
			"discarded generation",
			format!(
				"nothing being discarded references a generation newer than the loop state's {generation}, so the `Lost` signal sent after the advance covers everything still running"
			),
		);

		Ok((
			builder.finish(RepairState::Ready),
			Some(RepairPlan::AdvanceLoopIteration {
				loop_location,
				new_iteration: highest,
				previous_iteration: iteration,
				generation,
			}),
		))
	}

	async fn scan_active_history(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<BTreeMap<Location, RawEvent>> {
		self.scan_raw_history(
			tx,
			&self
				.subspace
				.subspace(&keys::history::HistorySubspaceKey::new(
					workflow_id,
					keys::history::HistorySubspaceVariant::Active,
				)),
		)
		.await
	}

	/// Validates that the workflow died replaying a `propose` output the current code treats as
	/// unreachable, and that the event can be cleared without shifting replay.
	async fn inspect_propose_consensus_failed(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder =
			InspectionBuilder::new(RepairVariant::ProposeConsensusFailed, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		let Some(workflow_name) = meta.name.clone() else {
			return Ok((
				builder.refuse("workflow exists", "workflow not found"),
				None,
			));
		};
		if workflow_name != ACTOR2_WORKFLOW_NAME {
			return Ok((
				builder.refuse(
					"workflow name",
					format!(
						"is {workflow_name:?}, this repair only handles {ACTOR2_WORKFLOW_NAME:?}"
					),
				),
				None,
			));
		}
		builder.pass("workflow name", format!("{workflow_name:?}"));

		if let Some(refusal) = live_workflow_refusal(&meta) {
			return Ok((builder.refuse("workflow state", refusal), None));
		}
		builder.pass(
			"workflow state",
			"dead, not leased, no pending wake condition",
		);

		// Unlike the divergence defects this error does not drift. The workflow bails on an output
		// already in its history, so every replay dies exactly the same way.
		let error = meta.error.clone().unwrap_or_default();
		if !error.contains("unreachable: ConsensusFailed") {
			return Ok((
				builder.refuse(
					"workflow error",
					format!(
						"is {error:?}, this repair only handles `unreachable: ConsensusFailed`"
					),
				),
				None,
			));
		}
		builder.pass("workflow error", format!("{error:?}"));

		let active = self.scan_active_history(tx, workflow_id).await?;
		let Some((last_location, last_event)) = active.iter().next_back() else {
			return Ok((
				builder.refuse("active history", "workflow has no active history"),
				None,
			));
		};

		let Some((propose_location, propose)) = active
			.iter()
			.filter(|(_, event)| event.is_activity(PROPOSE_ACTIVITY))
			.next_back()
		else {
			return Ok((
				builder.refuse(
					"propose event",
					format!(
						"active history has no {PROPOSE_ACTIVITY:?} activity, the repair may already have cleared it"
					),
				),
				None,
			));
		};
		builder.location = Some(propose_location.clone());

		if !propose.has_output {
			return Ok((
				builder.refuse(
					"propose output",
					format!("{propose_location} has no output, the activity never completed"),
				),
				None,
			));
		}

		let output = propose.output();
		if PROPOSE_HANDLED_OUTPUTS
			.iter()
			.any(|handled| output.contains(handled))
		{
			return Ok((
				builder.refuse(
					"propose output",
					format!(
						"{propose_location} recorded {output}, which the workflow handles rather than bails on"
					),
				),
				None,
			));
		}
		builder.pass(
			"propose output",
			format!(
				"{propose_location} recorded {output}, which the workflow bails on as unreachable"
			),
		);

		if last_location != propose_location {
			return Ok((
				builder.manual(
					"trailing event",
					format!(
						"{propose_location} is followed by {} at {last_location}, so clearing it would shift every event after it",
						last_event.describe()
					),
				),
				None,
			));
		}
		builder.pass(
			"trailing event",
			format!(
				"{propose_location} is the last recorded event, so clearing it cannot shift replay"
			),
		);

		let plan = RepairPlan::ClearTrailingEvent {
			location: propose_location.clone(),
			keys: propose.keys.clone(),
			description: format!("activity {PROPOSE_ACTIVITY:?} (output was {output})"),
		};

		Ok((builder.finish(RepairState::Ready), Some(plan)))
	}

	/// Validates that a workflow still in actor creation lost its state key, and rebuilds the state
	/// its init activity recorded.
	async fn inspect_missing_init_state(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder = InspectionBuilder::new(RepairVariant::MissingInitState, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		let Some(workflow_name) = meta.name.clone() else {
			return Ok((
				builder.refuse("workflow exists", "workflow not found"),
				None,
			));
		};
		if workflow_name != ACTOR2_WORKFLOW_NAME {
			return Ok((
				builder.refuse(
					"workflow name",
					format!(
						"is {workflow_name:?}, this repair only handles {ACTOR2_WORKFLOW_NAME:?}"
					),
				),
				None,
			));
		}
		builder.pass("workflow name", format!("{workflow_name:?}"));

		if let Some(refusal) = live_workflow_refusal(&meta) {
			return Ok((builder.refuse("workflow state", refusal), None));
		}
		builder.pass(
			"workflow state",
			"dead, not leased, no pending wake condition",
		);

		let error = meta.error.clone().unwrap_or_default();
		if !error.contains("expected struct State") {
			return Ok((
				builder.refuse(
					"workflow error",
					format!(
						"is {error:?}, this repair only handles an activity that failed reading a null state"
					),
				),
				None,
			));
		}
		builder.pass("workflow error", format!("{error:?}"));

		if let Some(state) = &meta.state {
			return Ok((
				builder.refuse(
					"state key",
					format!("already holds {state}, there is nothing to rebuild"),
				),
				None,
			));
		}
		builder.pass(
			"state key",
			"absent, so every activity that reads state fails",
		);

		// State is not recorded in history and completed activities never re-run, so only a history
		// where the init activity is still the sole state writer can be rebuilt exactly. Creation
		// records one activity per top level coordinate, so any other shape means the workflow got
		// past creation and its state would have to be guessed.
		let active = self.scan_active_history(tx, workflow_id).await?;
		let expected_names: Vec<&str> = match active.len() {
			// The key lookup found an existing reservation, so the workflow skipped `propose`.
			3 => vec![
				INSERT_STATE_ACTIVITY,
				LOOKUP_KEY_ACTIVITY,
				RESERVE_ACTOR_KEY_ACTIVITY,
			],
			4 => vec![
				INSERT_STATE_ACTIVITY,
				LOOKUP_KEY_ACTIVITY,
				PROPOSE_ACTIVITY,
				RESERVE_ACTOR_KEY_ACTIVITY,
			],
			_ => Vec::new(),
		};

		let recorded = active
			.iter()
			.map(|(location, event)| format!("{location} {}", event.describe()))
			.collect::<Vec<_>>()
			.join(", ");

		let shape_matches = !expected_names.is_empty()
			&& active.iter().zip(expected_names.iter()).enumerate().all(
				|(i, ((location, event), name))| {
					location == &Location::empty().join(Coordinate::simple(i + 1))
						&& event.is_activity(name)
				},
			);
		if !shape_matches {
			return Ok((
				builder.refuse(
					"history shape",
					format!(
						"active history is [{recorded}], this repair only handles the creation activities before the actor is reserved"
					),
				),
				None,
			));
		}

		let (init_location, init) = active.iter().next().expect("checked len");
		let (last_location, last) = active.iter().next_back().expect("checked len");

		if active
			.iter()
			.take(active.len() - 1)
			.any(|(_, event)| !event.has_output)
		{
			return Ok((
				builder.refuse(
					"recorded outputs",
					"an activity before the last one has no output, so the workflow is mid creation rather than stuck on state",
				),
				None,
			));
		}
		if last.has_output {
			return Ok((
				builder.refuse(
					"recorded outputs",
					format!(
						"{last_location} already recorded an output, so it did not fail reading state"
					),
				),
				None,
			));
		}
		builder.pass(
			"history shape",
			format!("[{recorded}], with {last_location} recorded as failed"),
		);
		builder.location = Some(last_location.clone());

		let raw_input = init.input();
		let input: serde_json::Value = serde_json::from_str(&raw_input)
			.with_context(|| format!("failed to parse {init_location} input"))?;

		let Some(from_v1) = input.get("from_v1").and_then(|from_v1| from_v1.as_bool()) else {
			return Ok((
				builder.refuse(
					"activity input",
					format!("{init_location} input {raw_input} does not record from_v1"),
				),
				None,
			));
		};

		let (Some(actor_id), Some(name), Some(pool_name), Some(namespace_id)) = (
			input.get("actor_id").and_then(|v| v.as_str()),
			input.get("name").and_then(|v| v.as_str()),
			input.get("pool_name").and_then(|v| v.as_str()),
			input.get("namespace_id").and_then(|v| v.as_str()),
		) else {
			return Ok((
				builder.refuse(
					"activity input",
					format!("{init_location} input {raw_input} is missing a field the state needs"),
				),
				None,
			));
		};

		// `insert_state_and_db` writes its input timestamp for a new actor, and for a v1 actor reads
		// the one that actor's v1 workflow already wrote. Read the same pegboard key here so the
		// rebuilt state carries what the activity actually recorded. The read joins this
		// transaction's conflict ranges, so a concurrent write to it fails the repair rather than
		// racing it.
		let create_ts = if from_v1 {
			let Ok(actor_id) = Id::from_str(actor_id) else {
				return Ok((
					builder.refuse(
						"activity input",
						format!(
							"{init_location} input records actor_id {actor_id:?}, which is not an id"
						),
					),
					None,
				));
			};

			let create_ts_key =
				Subspace::new(&(RIVET, PEGBOARD)).pack(&(ACTOR, DATA, actor_id, CREATE_TS));

			let Some(entry) = tx.get(&create_ts_key, Serializable).await? else {
				return Ok((
					builder.manual(
						"actor create ts",
						format!(
							"v1 actor {actor_id} has no create_ts key, so the timestamp {init_location} recorded cannot be recovered"
						),
					),
					None,
				));
			};

			let raw: &[u8] = &entry;
			i64::from_be_bytes(
				raw.try_into()
					.with_context(|| format!("v1 actor {actor_id} create_ts is not an i64"))?,
			)
		} else {
			let Some(create_ts) = input.get("create_ts").and_then(|v| v.as_i64()) else {
				return Ok((
					builder.refuse(
						"activity input",
						format!("{init_location} input {raw_input} does not record create_ts"),
					),
					None,
				));
			};

			create_ts
		};
		builder.pass(
			"actor create ts",
			if from_v1 {
				format!("{create_ts}, read from the v1 actor's pegboard key")
			} else {
				format!("{create_ts}, from the {init_location} input")
			},
		);

		let key = input.get("key").cloned().unwrap_or(serde_json::Value::Null);
		if !key.is_string() && !key.is_null() {
			return Ok((
				builder.refuse(
					"activity input",
					format!("{init_location} input records key {key}, expected a string or null"),
				),
				None,
			));
		}

		// Mirrors `pegboard_actor2::State::new`, which derives every field from this input plus the
		// create timestamp. Nothing re-runs, so the counters the init activity already moved stay
		// where they are.
		let state = serde_json::json!({
			"actor_id": actor_id,
			"name": name,
			"pool_name": pool_name,
			"key": key,
			"namespace_id": namespace_id,
			"acquired_slot": false,
			"envoy_last_command_idx": -1,
			"envoy_key": null,
			"create_ts": create_ts,
			"create_complete_ts": null,
			"allocate_ts": null,
			"start_ts": null,
			"sleep_ts": null,
			"connectable_ts": null,
			"reschedule_ts": null,
			"destroy_ts": null,
			"error": null,
		})
		.to_string();
		builder.pass(
			"rebuilt state",
			format!("{state}, derived from the {init_location} input"),
		);

		let plan = RepairPlan::WriteInitState {
			location: last_location.clone(),
			keys: last.keys.clone(),
			state,
		};

		Ok((builder.finish(RepairState::Ready), Some(plan)))
	}

	/// Validates the `deallocate` / `set_error` divergence and locates the slot the missing
	/// `set_error` activity belongs in.
	async fn inspect_deallocate_set_error(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder = InspectionBuilder::new(RepairVariant::DeallocateSetError, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		let Some(workflow_name) = meta.name.clone() else {
			return Ok((
				builder.refuse("workflow exists", "workflow not found"),
				None,
			));
		};
		if workflow_name != ACTOR2_WORKFLOW_NAME {
			return Ok((
				builder.refuse(
					"workflow name",
					format!(
						"is {workflow_name:?}, this repair only handles {ACTOR2_WORKFLOW_NAME:?}"
					),
				),
				None,
			));
		}
		builder.pass("workflow name", format!("{workflow_name:?}"));

		// This repair keys entirely off the exact divergence message. Unlike the orphaned sleep
		// state defect this error does not drift, and the location it names is the only reliable
		// way to tell which `deallocate` diverged.
		let Some(target_location) = meta.error.as_deref().and_then(parse_deallocate_divergence)
		else {
			return Ok((
				builder.refuse(
					"workflow error",
					format!(
						"is {:?}, this repair only handles `history diverged: expected activity \"deallocate\" at {{...}}, found activity \"set_error\"`",
						meta.error.as_deref().unwrap_or("<none>")
					),
				),
				None,
			));
		};
		builder.pass(
			"workflow error",
			format!(
				"diverged at {target_location}, expected {DEALLOCATE_ACTIVITY:?} found {SET_ERROR_ACTIVITY:?}"
			),
		);

		if let Some(reason) = live_workflow_refusal(&meta) {
			return Ok((builder.refuse("workflow state", reason), None));
		}
		builder.pass(
			"workflow state",
			"dead, not leased, no pending wake condition",
		);

		let root = target_location.root();
		let branch = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::EventHistorySubspaceKey::entire(
						workflow_id,
						root.clone(),
						false,
					)),
			)
			.await?;

		let Some(target) = branch.get(&target_location) else {
			return Ok((
				builder.refuse(
					"target event",
					format!("no event exists at {target_location}"),
				),
				None,
			));
		};
		if !target.is_activity(DEALLOCATE_ACTIVITY) {
			return Ok((
				builder.refuse(
					"target event",
					format!(
						"{target_location} is {}, expected activity {DEALLOCATE_ACTIVITY:?}",
						target.describe()
					),
				),
				None,
			));
		}
		let Some(version) = target.version else {
			return Ok((
				builder.refuse("target event", format!("{target_location} has no version")),
				None,
			));
		};
		if target.create_ts.is_none() {
			return Ok((
				builder.refuse(
					"target event",
					format!("{target_location} has no create timestamp"),
				),
				None,
			));
		}
		builder.pass(
			"target event",
			format!("{target_location} is activity {DEALLOCATE_ACTIVITY:?} v{version}"),
		);

		// Only direct children of the root form the branch gasoline replays. Deeper descendants of
		// sibling events are in the scan but are not part of this cursor's sequence.
		let siblings = branch
			.iter()
			.filter(|(location, event)| location.root() == root && event.event_type.is_some())
			.map(|(location, event)| (location.clone(), event))
			.collect::<Vec<_>>();
		let target_idx = siblings
			.iter()
			.position(|(location, _)| location == &target_location)
			.context("target event missing from its own branch")?;
		let target_coord = target_location.tail().context("empty target location")?;

		let predecessor_coord = target_idx
			.checked_sub(1)
			.and_then(|idx| siblings[idx].0.tail())
			.cloned();

		// A previous run of this repair leaves the inserted activity directly before the target, at
		// the coordinate the cursor would have picked for it. Recognize it rather than inserting a
		// second copy.
		let mut existing_location = None;
		if target_idx > 0 {
			let (candidate_location, candidate) = &siblings[target_idx - 1];
			let prior_coord = target_idx
				.checked_sub(2)
				.and_then(|idx| siblings[idx].0.tail())
				.cloned();

			if candidate_location.tail()
				== Some(&insertion_coordinate(prior_coord.as_ref(), target_coord))
				&& is_inserted_set_error(candidate, version)
			{
				existing_location = Some(candidate_location.clone());
			}
		}

		let insertion_location = match existing_location {
			Some(location) => location,
			None => {
				let coord = insertion_coordinate(predecessor_coord.as_ref(), target_coord);

				// Gasoline replays a branch in coordinate order, so an event that does not sort
				// strictly between its predecessor and the target would diverge all over again.
				if predecessor_coord.is_some_and(|previous| coord <= previous)
					|| &coord >= target_coord
				{
					return Ok((
						builder.refuse(
							"insertion slot",
							format!(
								"computed coordinate {coord} does not sort strictly between the predecessor and {target_coord}"
							),
						),
						None,
					));
				}

				root.join(coord)
			}
		};
		builder.location = Some(insertion_location.clone());

		let occupant = branch.get(&insertion_location);

		if let Some(occupant) = occupant {
			if !is_inserted_set_error(occupant, version) {
				return Ok((
					builder.refuse(
						"insertion slot",
						format!(
							"{insertion_location} is occupied by {}, which this repair did not write",
							occupant.describe()
						),
					),
					None,
				));
			}

			if occupant.has_output {
				return Ok((
					builder.refuse(
						"insertion slot",
						format!(
							"{insertion_location} already holds a completed {SET_ERROR_ACTIVITY:?}, so this workflow already replayed past the repair and its remaining error is unrelated"
						),
					),
					None,
				));
			}

			builder.pass(
				"insertion slot",
				format!("{insertion_location} already holds the inserted {SET_ERROR_ACTIVITY:?}, waiting to be replayed"),
			);

			return Ok((builder.finish(RepairState::AlreadyApplied), None));
		}

		builder.pass(
			"insertion slot",
			format!("{insertion_location} is free and sorts before {target_location}"),
		);

		Ok((
			builder.finish(RepairState::Ready),
			Some(RepairPlan::DeallocateSetError {
				insertion_location,
				version,
			}),
		))
	}

	/// Validates an orphaned active sleep-state row and the complete forgotten Sleep event that
	/// makes clearing it safe.
	async fn inspect_orphaned_sleep_state(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Option<Location>,
	) -> Result<(RepairInspection, Option<RepairPlan>)> {
		let mut builder = InspectionBuilder::new(RepairVariant::OrphanedSleepState, workflow_id);

		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		builder.apply_meta(&meta);

		let Some(workflow_name) = meta.name.clone() else {
			return Ok((
				builder.refuse("workflow exists", "workflow not found"),
				None,
			));
		};
		if workflow_name != ACTOR2_WORKFLOW_NAME {
			return Ok((
				builder.refuse(
					"workflow name",
					format!(
						"is {workflow_name:?}, this repair only handles {ACTOR2_WORKFLOW_NAME:?}"
					),
				),
				None,
			));
		}
		builder.pass("workflow name", format!("{workflow_name:?}"));

		// The persisted error is reported but never gated on. It drifts for this defect, commonly
		// from `missing event data: event_type` to `workflow evicted`, so refusing on it would
		// reject real targets. The history shape is the only trustworthy signal.
		builder.pass(
			"workflow error",
			format!(
				"{:?}, not used to select this repair because it drifts for this defect",
				meta.error.as_deref().unwrap_or("<none>")
			),
		);

		if meta.is_silenced {
			return Ok((
				builder.refuse("workflow state", "workflow is silenced"),
				None,
			));
		}
		if meta.is_complete {
			return Ok((
				builder.refuse("workflow state", "workflow is complete"),
				None,
			));
		}

		// A lease is reported but not refused. The orphan makes the history undecodable, so
		// `pull_workflow_history` always fails and the workflow is never installed into a worker's
		// running set. The lease transaction commits before the separate history decode
		// transaction fails, so a lease here is a stranded record kept alive by the owning worker's
		// ping, not a live workflow. `pull_workflows` skips any workflow that still holds a lease,
		// so the repair has to release it or the workflow could never be re-pulled.
		builder.pass(
			"workflow lease",
			if meta.has_lease || meta.has_worker {
				"stranded lease present, the repair releases it so the workflow can be re-pulled"
					.to_string()
			} else {
				"none".to_string()
			},
		);

		// Read once and reuse. The orphan is found here, its row is validated below, and the trailing
		// check needs to see everything recorded after it.
		let active = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::HistorySubspaceKey::new(
						workflow_id,
						keys::history::HistorySubspaceVariant::Active,
					)),
			)
			.await?;

		let orphan_location = match &location {
			Some(location) => location.clone(),
			None => {
				let orphans = active
					.iter()
					.filter(|(_, event)| event.is_sleep_orphan())
					.map(|(location, _)| location.clone())
					.collect::<Vec<_>>();

				match orphans.len() {
					0 => {
						return Ok((
							builder.refuse(
								"orphan scan",
								"active history has no orphaned sleep-state row",
							),
							None,
						));
					}
					1 => orphans.into_iter().next().expect("checked len"),
					_ => {
						let list = orphans
							.iter()
							.map(ToString::to_string)
							.collect::<Vec<_>>()
							.join(", ");

						return Ok((
							builder.refuse(
								"orphan scan",
								format!(
									"active history has {} orphaned sleep-state rows ({list}), pass --location to repair exactly one at a time",
									orphans.len()
								),
							),
							None,
						));
					}
				}
			}
		};
		builder.location = Some(orphan_location.clone());

		// The defect only produces sleep sub-events of a loop iteration, shaped {loop, iteration, 1}.
		let Some(iteration_coord) = orphan_location
			.root()
			.tail()
			.cloned()
			.filter(|_| orphan_location.tail() == Some(&Coordinate::simple(1)))
		else {
			return Ok((
				builder.refuse(
					"orphan location",
					format!("{orphan_location} is not shaped {{loop, iteration, 1}}"),
				),
				None,
			));
		};
		builder.pass(
			"orphan location",
			format!("{orphan_location} is a loop iteration sleep sub-event"),
		);

		// The forgotten copy lives under {loop, iteration}, so read just that one subspace rather than
		// scanning the whole forgotten history.
		let iteration_root = orphan_location.root().root();
		let forgotten = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::EventHistorySubspaceKey::new(
						workflow_id,
						iteration_root,
						iteration_coord.head(),
						true,
					)),
			)
			.await?;

		let forgotten_event = forgotten.get(&orphan_location);
		let forgotten_complete = forgotten_event.is_some_and(RawEvent::is_complete_sleep);

		let Some(active_event) = active.get(&orphan_location) else {
			if forgotten_complete {
				builder.pass(
					"orphan row",
					format!("{orphan_location} is already cleared from active history"),
				);

				return Ok((builder.finish(RepairState::AlreadyApplied), None));
			}

			return Ok((
				builder.refuse(
					"orphan row",
					format!("{orphan_location} has no active row and no complete forgotten Sleep event, this workflow does not have the defect"),
				),
				None,
			));
		};

		if !active_event.is_sleep_orphan() {
			return Ok((
				builder.refuse(
					"orphan row",
					format!(
						"{orphan_location} holds {}, which is more than a bare sleep_state, so clearing it would lose data",
						active_event.describe()
					),
				),
				None,
			));
		}
		let sleep_state = active_event
			.sleep_state
			.expect("checked by is_sleep_orphan");
		builder.pass(
			"orphan row",
			format!("{orphan_location} holds only sleep_state ({sleep_state}) and no event_type"),
		);

		let Some(forgotten_event) = forgotten_event else {
			return Ok((
				builder.refuse(
					"forgotten copy",
					format!("{orphan_location} has no forgotten event, clearing the active row would lose the Sleep event entirely"),
				),
				None,
			));
		};
		if !forgotten_complete {
			return Ok((
				builder.refuse(
					"forgotten copy",
					format!(
						"{orphan_location} forgotten event is {}, not a complete Sleep event",
						forgotten_event.describe()
					),
				),
				None,
			));
		}
		builder.pass(
			"forgotten copy",
			format!(
				"{orphan_location} holds a complete Sleep event in forgotten history (deadline {}, state {})",
				forgotten_event
					.deadline_ts
					.expect("checked by is_complete_sleep"),
				forgotten_event
					.sleep_state
					.expect("checked by is_complete_sleep"),
			),
		);

		// Replay walks the history by index, so clearing a row that is not the last one shifts every
		// event recorded after it. The workflow then resumes live inside an iteration that already has
		// recorded events and dies with `history diverged` once the loop reaches the next one. Which
		// side is authoritative depends on what those later iterations recorded, so report it and stop
		// rather than guessing.
		let Some((last_location, last_event)) = active.iter().next_back() else {
			return Ok((
				builder.refuse("trailing event", "active history is empty"),
				None,
			));
		};
		if last_location != &orphan_location {
			return Ok((
				builder.manual(
					"trailing event",
					format!(
						"{orphan_location} is followed by {} at {last_location}, so clearing it would shift every event after it",
						last_event.describe()
					),
				),
				None,
			));
		}
		builder.pass(
			"trailing event",
			format!(
				"{orphan_location} is the last recorded event, so clearing it cannot shift replay"
			),
		);

		Ok((
			builder.finish(RepairState::Ready),
			Some(RepairPlan::OrphanedSleepState {
				location: orphan_location,
				orphan_keys: active_event.keys.clone(),
				orphan_sleep_state: sleep_state,
				workflow_name,
			}),
		))
	}

	/// Moves one loop branch out of active history into forgotten, in its own transaction. This is
	/// what `upsert_workflow_loop_event` does for a whole range at once, scoped to a single branch
	/// so the transaction stays small.
	async fn move_branch_to_forgotten(&self, workflow_id: Id, branch: &Location) -> Result<usize> {
		let loop_location = branch.root();
		let coord = branch.tail().context("empty branch location")?.head();

		self.pools
			.udb()?
			.txn("gas_debug_repair_drain_branch", |tx| {
				let loop_location = loop_location.clone();

				async move {
					let active = self
						.subspace
						.subspace(&keys::history::HistorySubspaceKey::new(
							workflow_id,
							keys::history::HistorySubspaceVariant::Active,
						));
					let forgotten =
						self.subspace
							.subspace(&keys::history::HistorySubspaceKey::new(
								workflow_id,
								keys::history::HistorySubspaceVariant::Forgotten,
							));
					let (start, end) = self
						.subspace
						.subspace(&keys::history::EventHistorySubspaceKey::new(
							workflow_id,
							loop_location.clone(),
							coord,
							false,
						))
						.range();

					let mut stream = tx.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(start.as_slice(), end.as_slice()).into()
						},
						Serializable,
					);

					let mut moved = 0;
					loop {
						let Some(entry) = stream.try_next().await? else {
							break;
						};

						if !active.is_start_of(entry.key()) {
							bail!("history key outside the active subspace");
						}

						// Same prefix rewrite loop compaction does.
						let truncated = &entry.key()[active.bytes().len()..];
						tx.set(&[forgotten.bytes(), truncated].concat(), entry.value());
						moved += 1;
					}

					tx.clear_range(&start, &end);

					Ok(moved)
				}
			})
			.await
	}

	async fn apply_plan(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		plan: RepairPlan,
	) -> Result<(PostCommit, Vec<String>)> {
		let tx = tx.with_subspace(self.subspace.clone());
		let mut mutations = Vec::new();

		match plan {
			RepairPlan::DeallocateSetError {
				insertion_location,
				version,
			} => {
				let create_ts = rivet_util::timestamp::now();

				tx.write(
					&keys::history::EventTypeKey::new(workflow_id, insertion_location.clone()),
					EventType::Activity,
				)?;
				tx.write(
					&keys::history::VersionKey::new(workflow_id, insertion_location.clone()),
					version,
				)?;
				tx.write(
					&keys::history::CreateTsKey::new(workflow_id, insertion_location.clone()),
					create_ts,
				)?;
				tx.write(
					&keys::history::NameKey::new(workflow_id, insertion_location.clone()),
					SET_ERROR_ACTIVITY.to_string(),
				)?;

				// Written with input but no output, which is what makes it incomplete. Gasoline
				// runs an activity that has no output on the next wake, then replays the original
				// `deallocate` and the unchanged history tail.
				let input_key =
					keys::history::InputKey::new(workflow_id, insertion_location.clone());
				let input = serde_json::value::RawValue::from_string(SET_ERROR_INPUT.to_string())
					.context("failed to build set_error input")?;
				for (i, chunk) in input_key.split_ref(&input)?.into_iter().enumerate() {
					tx.set(&tx.pack(&input_key.chunk(i)), &chunk);
				}

				mutations.push(format!(
					"set {insertion_location} activity {SET_ERROR_ACTIVITY:?} v{version} create_ts={create_ts} input={SET_ERROR_INPUT}"
				));

				Ok((PostCommit::default(), mutations))
			}
			RepairPlan::OrphanedSleepState {
				location,
				orphan_keys,
				orphan_sleep_state,
				workflow_name,
			} => {
				// Clear the exact keys the inspection validated rather than a range, so a row that
				// appeared between the scan and this write is never silently destroyed.
				for key in &orphan_keys {
					tx.clear(key);
				}
				mutations.push(format!(
					"clear {location} sleep_state (was {orphan_sleep_state})"
				));

				// Release the stranded lease so `pull_workflows` stops skipping this workflow, and
				// re-arm it exactly the way lease failover does. Doing it here rather than through
				// `wake_workflows` keeps the workflow state metric consistent: a leased workflow is
				// counted as active, not dead.
				let has_lease = tx
					.exists(&keys::workflow::LeaseKey::new(workflow_id), Serializable)
					.await?;
				let has_worker = tx
					.exists(&keys::workflow::WorkerIdKey::new(workflow_id), Serializable)
					.await?;

				if !has_lease && !has_worker {
					return Ok((PostCommit::default(), mutations));
				}

				tx.delete(&keys::workflow::LeaseKey::new(workflow_id));
				tx.delete(&keys::workflow::WorkerIdKey::new(workflow_id));
				tx.write(
					&keys::wake::WorkflowWakeConditionKey::new(
						workflow_name.clone(),
						workflow_id,
						keys::wake::WakeCondition::Immediate,
					),
					(),
				)?;

				update_metric(
					&tx,
					Some(keys::metric::Metric::WorkflowActive(workflow_name.clone())),
					Some(keys::metric::Metric::WorkflowSleeping(workflow_name)),
				);

				mutations.push("clear stranded lease and worker id".to_string());
				mutations.push("set immediate wake condition".to_string());

				Ok((
					PostCommit {
						wake_armed: true,
						..Default::default()
					},
					mutations,
				))
			}
			RepairPlan::AdvanceLoopIteration {
				loop_location,
				new_iteration,
				previous_iteration,
				generation,
			} => {
				// Only the counter moves. The loop's state is left alone, and the `Lost` signal
				// sent after this commits is what tears that state down.
				tx.write(
					&keys::history::IterationKey::new(workflow_id, loop_location.clone()),
					new_iteration,
				)?;

				mutations.push(format!(
					"set {loop_location} iteration {previous_iteration} -> {new_iteration}"
				));

				// `upsert_workflow_loop_event` compacts everything below the loop's iteration in one
				// transaction, and it scans from coordinate 0. Advancing the counter in one jump
				// would leave every skipped branch for that single transaction to move, which can
				// exceed the transaction size or time limit and wedge the workflow with no way back.
				// Drain them here instead, one branch per transaction, so that compaction finds
				// nothing left to do.
				let drain_branches = (previous_iteration + 1..=new_iteration)
					.map(|coord| loop_location.join(Coordinate::simple(coord)))
					.collect::<Vec<_>>();

				Ok((
					PostCommit {
						lost_generation: Some(generation),
						drain_branches,
						..Default::default()
					},
					mutations,
				))
			}
			RepairPlan::ClearTrailingEvent {
				location,
				keys,
				description,
			} => {
				// Clear the exact keys the inspection validated rather than a range, so a row that
				// appeared between the scan and this write is never silently destroyed.
				for key in &keys {
					tx.clear(key);
				}
				mutations.push(format!("clear {location} {description}"));

				Ok((PostCommit::default(), mutations))
			}
			RepairPlan::WriteInitState {
				location,
				keys,
				state,
			} => {
				let state_key = keys::workflow::StateKey::new(workflow_id);
				let state_value = serde_json::value::RawValue::from_string(state.clone())
					.context("failed to build workflow state")?;
				for (i, chunk) in state_key.split_ref(&state_value)?.into_iter().enumerate() {
					tx.set(&tx.pack(&state_key.chunk(i)), &chunk);
				}
				mutations.push(format!("set workflow state {state}"));

				// Clearing the trailing activity resets its error rows, so it runs again with a
				// fresh retry budget rather than dying on the first transient failure.
				for key in &keys {
					tx.clear(key);
				}
				mutations.push(format!(
					"clear {location} activity {RESERVE_ACTOR_KEY_ACTIVITY:?}"
				));

				Ok((PostCommit::default(), mutations))
			}
		}
	}

	async fn verify_deallocate_set_error(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Location,
	) -> Result<RepairVerification> {
		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		let mut checks = Vec::new();

		let root = location.root();
		let active = self.scan_branch(tx, workflow_id, &root, false).await?;

		// A successful replay makes the loop commit its next iteration, and that commit moves every
		// event up to that iteration into forgotten history and clears the active range. The anchor
		// leaving active history is therefore what this repair working looks like, so read forgotten
		// history before concluding anything about a missing event.
		let (branch, compacted) = if active.contains_key(&location) {
			(active, false)
		} else {
			(self.scan_branch(tx, workflow_id, &root, true).await?, true)
		};

		let inserted = branch.get(&location);
		let inserted_present = inserted.is_some_and(|event| {
			event.is_activity(SET_ERROR_ACTIVITY) && event.input() == SET_ERROR_INPUT
		});
		let inserted_has_output = inserted.is_some_and(|event| event.has_output);

		// The whole point of the repair is that the original event survives, so confirm the next
		// event in the branch is still the `deallocate` the inserted activity runs before.
		let successor = branch
			.iter()
			.filter(|(candidate, event)| {
				candidate.root() == root && **candidate > location && event.event_type.is_some()
			})
			.map(|(_, event)| event)
			.next();
		let deallocate_present =
			successor.is_some_and(|event| event.is_activity(DEALLOCATE_ACTIVITY));

		if compacted {
			checks.push(RepairCheck::pass(
				"loop progress",
				format!(
					"{location} left active history, so the loop committed a later iteration and replay got past the repaired branch"
				),
			));
			checks.push(RepairCheck::pass(
				"forgotten branch",
				if inserted_present && deallocate_present {
					format!(
						"{location} is activity {SET_ERROR_ACTIVITY:?} followed by activity {DEALLOCATE_ACTIVITY:?} in forgotten history"
					)
				} else {
					// Forgotten history only retains the most recent iterations, so a branch this
					// old can be pruned entirely.
					format!("{location} is no longer retained in forgotten history")
				},
			));
		} else {
			if inserted_present {
				checks.push(RepairCheck::pass(
					"inserted event",
					format!(
						"{location} is activity {SET_ERROR_ACTIVITY:?} and has {}",
						if inserted_has_output {
							"run"
						} else {
							"not run yet"
						}
					),
				));
			} else {
				checks.push(RepairCheck::fail(
					"inserted event",
					format!(
						"{location} is {}, expected activity {SET_ERROR_ACTIVITY:?}",
						inserted
							.map(RawEvent::describe)
							.unwrap_or_else(|| "absent".to_string())
					),
				));
			}

			if deallocate_present {
				checks.push(RepairCheck::pass(
					"original deallocate",
					format!("still present directly after {location}"),
				));
			} else {
				checks.push(RepairCheck::fail(
					"original deallocate",
					format!(
						"the event after {location} is {}, expected activity {DEALLOCATE_ACTIVITY:?}",
						successor
							.map(RawEvent::describe)
							.unwrap_or_else(|| "absent".to_string())
					),
				));
			}
		}

		// A workflow keeps its pre-repair error until it next commits, so this only distinguishes a
		// fresh divergence from the stale one once something proves a commit happened.
		let diverged_again = meta
			.error
			.as_deref()
			.and_then(parse_deallocate_divergence)
			.is_some();
		let committed = compacted || inserted_has_output;

		// A leased workflow is still replaying, so its history and error are both mid-flight. That
		// has to outrank every conclusion drawn from them or the caller stops polling too early.
		let state = if meta.has_lease || meta.has_worker {
			RepairVerifyState::ReplayRunning
		} else if diverged_again && committed {
			RepairVerifyState::Regressed
		} else if compacted {
			classify_replayed_workflow(&meta)
		} else if !inserted_present || !deallocate_present {
			RepairVerifyState::Regressed
		} else if !inserted_has_output {
			RepairVerifyState::AwaitingReplay
		} else {
			classify_replayed_workflow(&meta)
		};

		Ok(RepairVerification {
			variant: RepairVariant::DeallocateSetError,
			workflow_id,
			workflow_error: meta.error.clone(),
			state,
			location,
			has_lease: meta.has_lease,
			has_worker: meta.has_worker,
			has_wake_condition: meta.has_wake_condition,
			checks,
		})
	}

	/// Reads one branch of a workflow's history, either the active copy or the forgotten copy a
	/// loop iteration commit moves it to.
	async fn scan_branch(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		root: &Location,
		forgotten: bool,
	) -> Result<BTreeMap<Location, RawEvent>> {
		self.scan_raw_history(
			tx,
			&self
				.subspace
				.subspace(&keys::history::EventHistorySubspaceKey::entire(
					workflow_id,
					root.clone(),
					forgotten,
				)),
		)
		.await
	}

	async fn verify_orphaned_sleep_state(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Location,
	) -> Result<RepairVerification> {
		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		let mut checks = Vec::new();

		let iteration_root = location.root().root();
		let iteration_coord = location
			.root()
			.tail()
			.cloned()
			.context("orphan location is not shaped {loop, iteration, 1}")?;

		let active = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::EventHistorySubspaceKey::new(
						workflow_id,
						iteration_root.clone(),
						iteration_coord.head(),
						false,
					)),
			)
			.await?;
		let forgotten = self
			.scan_raw_history(
				tx,
				&self
					.subspace
					.subspace(&keys::history::EventHistorySubspaceKey::new(
						workflow_id,
						iteration_root,
						iteration_coord.head(),
						true,
					)),
			)
			.await?;

		let orphan_present = active.get(&location).is_some_and(RawEvent::is_sleep_orphan);
		let forgotten_complete = forgotten
			.get(&location)
			.is_some_and(RawEvent::is_complete_sleep);

		if orphan_present {
			checks.push(RepairCheck::fail(
				"orphan row",
				format!("{location} still holds a bare sleep_state row"),
			));
		} else {
			checks.push(RepairCheck::pass(
				"orphan row",
				format!("{location} is cleared from active history"),
			));
		}

		if forgotten_complete {
			checks.push(RepairCheck::pass(
				"forgotten copy",
				format!("{location} still holds the complete Sleep event"),
			));
		} else {
			checks.push(RepairCheck::fail(
				"forgotten copy",
				format!("{location} no longer holds a complete Sleep event"),
			));
		}

		let state = if orphan_present {
			RepairVerifyState::Regressed
		} else if meta.has_lease || meta.has_worker {
			RepairVerifyState::ReplayRunning
		} else if meta
			.error
			.as_deref()
			.is_some_and(|error| error.contains("missing event data"))
		{
			RepairVerifyState::Regressed
		} else {
			classify_replayed_workflow(&meta)
		};

		Ok(RepairVerification {
			variant: RepairVariant::OrphanedSleepState,
			workflow_id,
			workflow_error: meta.error.clone(),
			state,
			location,
			has_lease: meta.has_lease,
			has_worker: meta.has_worker,
			has_wake_condition: meta.has_wake_condition,
			checks,
		})
	}

	async fn verify_loop_iteration_mismatch(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Location,
	) -> Result<RepairVerification> {
		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		let mut checks = Vec::new();

		let active = self.scan_active_history(tx, workflow_id).await?;
		let iteration = active.get(&location).and_then(|event| event.iteration);

		let resume = iteration.map(|iteration| location.join(Coordinate::simple(iteration + 1)));
		match &resume {
			Some(resume) => checks.push(RepairCheck::pass(
				"loop event",
				format!("{location} resumes at branch {resume}"),
			)),
			None => checks.push(RepairCheck::fail(
				"loop event",
				format!("{location} has no iteration counter"),
			)),
		}

		// Replay walks forward from the resume branch, so the repair only helped if the failure is
		// no longer anchored at or after it.
		let still_stuck = resume.as_ref().is_some_and(|resume| {
			meta.error
				.as_deref()
				.and_then(parse_error_location)
				.is_some_and(|error_location| {
					error_location.len() > location.len()
						&& error_location.starts_with(&location[..])
						&& resume.tail().is_some_and(|tail| {
							error_location[location.len()].head() >= tail.head()
						})
				})
		});

		if still_stuck {
			checks.push(RepairCheck::fail(
				"replay position",
				"replay is failing inside the loop's recorded iterations again",
			));
		} else {
			checks.push(RepairCheck::pass(
				"replay position",
				"replay is no longer failing inside the loop's recorded iterations",
			));
		}

		let state = if meta.has_lease || meta.has_worker {
			RepairVerifyState::ReplayRunning
		} else if resume.is_none() || still_stuck {
			RepairVerifyState::Regressed
		} else {
			classify_replayed_workflow(&meta)
		};

		Ok(RepairVerification {
			variant: RepairVariant::LoopIterationMismatch,
			workflow_id,
			workflow_error: meta.error.clone(),
			state,
			location,
			has_lease: meta.has_lease,
			has_worker: meta.has_worker,
			has_wake_condition: meta.has_wake_condition,
			checks,
		})
	}

	async fn verify_propose_consensus_failed(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Location,
	) -> Result<RepairVerification> {
		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		let mut checks = Vec::new();

		let active = self.scan_active_history(tx, workflow_id).await?;
		let event = active.get(&location);
		let unhandled_output = event.is_some_and(|event| {
			event.is_activity(PROPOSE_ACTIVITY)
				&& event.has_output
				&& !PROPOSE_HANDLED_OUTPUTS
					.iter()
					.any(|handled| event.output().contains(handled))
		});

		match event {
			Some(event) if unhandled_output => checks.push(RepairCheck::fail(
				"propose output",
				format!("{location} recorded {} again", event.output()),
			)),
			Some(event) if event.has_output => checks.push(RepairCheck::pass(
				"propose output",
				format!("{location} re-ran and recorded {}", event.output()),
			)),
			Some(_) => checks.push(RepairCheck::pass(
				"propose event",
				format!("{location} is recorded without an output, so the activity is running"),
			)),
			None => checks.push(RepairCheck::pass(
				"propose event",
				format!("{location} is cleared, so the activity runs on the next wake"),
			)),
		}

		let state = if unhandled_output {
			RepairVerifyState::Regressed
		} else if meta.has_lease || meta.has_worker {
			RepairVerifyState::ReplayRunning
		} else if meta
			.error
			.as_deref()
			.is_some_and(|error| error.contains("unreachable: ConsensusFailed"))
		{
			RepairVerifyState::Regressed
		} else {
			classify_replayed_workflow(&meta)
		};

		Ok(RepairVerification {
			variant: RepairVariant::ProposeConsensusFailed,
			workflow_id,
			workflow_error: meta.error.clone(),
			state,
			location,
			has_lease: meta.has_lease,
			has_worker: meta.has_worker,
			has_wake_condition: meta.has_wake_condition,
			checks,
		})
	}

	async fn verify_missing_init_state(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
		location: Location,
	) -> Result<RepairVerification> {
		let meta = self.read_workflow_meta(tx, workflow_id).await?;
		let mut checks = Vec::new();

		match &meta.state {
			Some(state) => checks.push(RepairCheck::pass("state key", format!("holds {state}"))),
			None => checks.push(RepairCheck::fail("state key", "is still absent")),
		}

		let active = self.scan_active_history(tx, workflow_id).await?;
		match active.get(&location) {
			Some(event) if event.has_output => checks.push(RepairCheck::pass(
				RESERVE_ACTOR_KEY_ACTIVITY,
				format!("{location} re-ran and recorded {}", event.output()),
			)),
			Some(_) => checks.push(RepairCheck::pass(
				RESERVE_ACTOR_KEY_ACTIVITY,
				format!("{location} is recorded without an output, so the activity is running"),
			)),
			None => checks.push(RepairCheck::pass(
				RESERVE_ACTOR_KEY_ACTIVITY,
				format!("{location} is cleared, so the activity runs on the next wake"),
			)),
		}

		let verify_state = if meta.state.is_none() {
			RepairVerifyState::Regressed
		} else if meta.has_lease || meta.has_worker {
			RepairVerifyState::ReplayRunning
		} else if meta
			.error
			.as_deref()
			.is_some_and(|error| error.contains("expected struct State"))
		{
			RepairVerifyState::Regressed
		} else {
			classify_replayed_workflow(&meta)
		};

		Ok(RepairVerification {
			variant: RepairVariant::MissingInitState,
			workflow_id,
			workflow_error: meta.error.clone(),
			state: verify_state,
			location,
			has_lease: meta.has_lease,
			has_worker: meta.has_worker,
			has_wake_condition: meta.has_wake_condition,
			checks,
		})
	}

	async fn read_workflow_meta(
		&self,
		tx: &universaldb::RetryableTransaction,
		workflow_id: Id,
	) -> Result<WorkflowMeta> {
		let tx = tx.with_subspace(self.subspace.clone());

		let name_key = keys::workflow::NameKey::new(workflow_id);
		let error_key = keys::workflow::ErrorKey::new(workflow_id);
		let lease_key = keys::workflow::LeaseKey::new(workflow_id);
		let worker_id_key = keys::workflow::WorkerIdKey::new(workflow_id);
		let has_wake_condition_key = keys::workflow::HasWakeConditionKey::new(workflow_id);
		let silence_ts_key = keys::workflow::SilenceTsKey::new(workflow_id);
		let output_key = keys::workflow::OutputKey::new(workflow_id);
		let output_subspace = self.subspace.subspace(&output_key);
		let state_key = keys::workflow::StateKey::new(workflow_id);
		let state_subspace = self.subspace.subspace(&state_key);

		let (
			name,
			error,
			has_lease,
			has_worker,
			has_wake_condition,
			is_silenced,
			is_complete,
			state,
		) = tokio::try_join!(
			tx.read_opt(&name_key, Serializable),
			tx.read_opt(&error_key, Serializable),
			tx.exists(&lease_key, Serializable),
			tx.exists(&worker_id_key, Serializable),
			tx.exists(&has_wake_condition_key, Serializable),
			tx.exists(&silence_ts_key, Serializable),
			async {
				tx.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::Exact,
						limit: Some(1),
						..(&output_subspace).into()
					},
					Serializable,
				)
				.try_next()
				.await
				.map(|entry| entry.is_some())
				.map_err(Into::into)
			},
			async {
				let chunks = tx
					.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(&state_subspace).into()
						},
						Serializable,
					)
					.try_collect::<Vec<_>>()
					.await?;

				Result::<_>::Ok((!chunks.is_empty()).then(|| {
					String::from_utf8_lossy(
						&chunks
							.iter()
							.flat_map(|chunk| chunk.value().to_vec())
							.collect::<Vec<_>>(),
					)
					.into_owned()
				}))
			},
		)?;

		Ok(WorkflowMeta {
			name,
			error,
			state,
			has_lease,
			has_worker,
			has_wake_condition,
			is_silenced,
			is_complete,
		})
	}

	/// Reads raw history rows grouped by location.
	///
	/// Deliberately does not go through `get_workflow_history`: these repairs run on workflows
	/// whose history cannot be decoded into events at all, which is the defect itself.
	async fn scan_raw_history(
		&self,
		tx: &universaldb::RetryableTransaction,
		subspace: &universaldb::tuple::Subspace,
	) -> Result<BTreeMap<Location, RawEvent>> {
		let mut events: BTreeMap<Location, RawEvent> = BTreeMap::new();
		let mut scanned = 0usize;

		let mut stream = tx.get_ranges_keyvalues(
			RangeOption {
				mode: StreamingMode::WantAll,
				..subspace.into()
			},
			Serializable,
		);

		loop {
			let Some(entry) = stream.try_next().await? else {
				break;
			};

			scanned += 1;
			ensure!(
				scanned <= MAX_SCANNED_KEYS,
				"history scan exceeded {MAX_SCANNED_KEYS} keys, refusing to repair"
			);

			let partial_key = self
				.subspace
				.unpack::<keys::history::PartialEventKey>(entry.key())?;
			let event = events.entry(partial_key.location).or_default();
			event.keys.push(entry.key().to_vec());

			if let Ok(key) = self
				.subspace
				.unpack::<keys::history::EventTypeKey>(entry.key())
			{
				event.event_type = Some(key.deserialize(entry.value())?);
				event.fields.push("event_type");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::VersionKey>(entry.key())
			{
				event.version = Some(key.deserialize(entry.value())?);
				event.fields.push("version");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::CreateTsKey>(entry.key())
			{
				event.create_ts = Some(key.deserialize(entry.value())?);
				event.fields.push("create_ts");
			} else if let Ok(key) = self.subspace.unpack::<keys::history::NameKey>(entry.key()) {
				event.name = Some(key.deserialize(entry.value())?);
				event.fields.push("name");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::DeadlineTsKey>(entry.key())
			{
				event.deadline_ts = Some(key.deserialize(entry.value())?);
				event.fields.push("deadline_ts");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::SleepStateKey>(entry.key())
			{
				event.sleep_state = Some(key.deserialize(entry.value())?);
				event.fields.push("sleep_state");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::IterationKey>(entry.key())
			{
				event.iteration = Some(key.deserialize(entry.value())?);
				event.fields.push("iteration");
			} else if let Ok(key) = self
				.subspace
				.unpack::<keys::history::IndexedNameKey>(entry.key())
			{
				event.signal_names.push(key.deserialize(entry.value())?);
				event.fields.push("signal_name");
			} else if self
				.subspace
				.unpack::<keys::history::InputChunkKey>(entry.key())
				.is_ok()
			{
				// Chunks arrive in key order, which is chunk order.
				event.input_chunks.push(entry.value().to_vec());
				event.fields.push("input");
			} else if self
				.subspace
				.unpack::<keys::history::OutputChunkKey>(entry.key())
				.is_ok()
			{
				// Chunks arrive in key order, which is chunk order.
				event.output_chunks.push(entry.value().to_vec());
				event.has_output = true;
				event.fields.push("output");
			} else {
				event.fields.push("other");
			}
		}

		Ok(events)
	}
}

/// Lists the events recorded strictly under a location, for side by side comparison.
fn describe_children(events: &BTreeMap<Location, RawEvent>, root: &Location) -> Vec<String> {
	events
		.iter()
		.filter(|(location, event)| {
			location.len() > root.len()
				&& location.starts_with(&root[..])
				&& event.event_type.is_some()
		})
		.map(|(location, event)| format!("{location} {}", event.describe()))
		.collect()
}

/// Whether a loop state's transition carries an envoy. The transition is either a bare string for a
/// unit variant or a single key object, and only some variants hold one.
fn transition_has_envoy(loop_state: &str) -> bool {
	serde_json::from_str::<serde_json::Value>(loop_state)
		.ok()
		.and_then(|state| state.get("transition").cloned())
		.and_then(|transition| Some(transition.as_object()?.values().next()?.clone()))
		.is_some_and(|variant| variant.get("envoy").is_some_and(|envoy| !envoy.is_null()))
}

/// Reads a string field out of a persisted JSON blob.
fn json_string_field(field: &str) -> impl Fn(&str) -> Option<String> + '_ {
	move |raw| {
		serde_json::from_str::<serde_json::Value>(raw)
			.ok()?
			.get(field)?
			.as_str()
			.map(ToString::to_string)
	}
}

/// Names of every signal consumed by a range of loop branches.
fn discarded_signal_names(
	events: &BTreeMap<Location, RawEvent>,
	loop_location: &Location,
	from: usize,
	to: usize,
) -> Vec<String> {
	(from..=to)
		.flat_map(|coord| {
			let branch = loop_location.join(Coordinate::simple(coord));

			events
				.iter()
				.filter(|(location, _)| location.root() == branch)
				.flat_map(|(_, event)| event.signal_names.clone())
				.collect::<Vec<_>>()
		})
		.collect()
}

/// Reads an actor generation out of a persisted JSON blob.
fn json_generation(raw: &str) -> Option<u64> {
	serde_json::from_str::<serde_json::Value>(raw)
		.ok()?
		.get("generation")?
		.as_u64()
}

/// Highest actor generation referenced by any activity input recorded in a range of loop branches.
fn max_referenced_generation(
	events: &BTreeMap<Location, RawEvent>,
	loop_location: &Location,
	from: usize,
	to: usize,
) -> Option<u64> {
	(from..=to)
		.filter_map(|coord| {
			let branch = loop_location.join(Coordinate::simple(coord));

			events
				.iter()
				.filter(|(location, _)| location.root() == branch)
				.filter_map(|(_, event)| json_generation(&event.input()))
				.max()
		})
		.max()
}

/// Reports the workflow name without gating on it. The detect-only symptoms are inconsistencies in
/// gasoline's own history primitives, so they are meaningful for any workflow.
fn refuse_if_missing(
	builder: &mut InspectionBuilder,
	meta: &WorkflowMeta,
) -> Option<RepairInspection> {
	match meta.name.as_deref() {
		None => Some(builder.refuse("workflow exists", "workflow not found")),
		Some(name) => {
			builder.pass("workflow", format!("{name:?}"));
			None
		}
	}
}

/// Parses the history location a replay failure is anchored at.
///
/// `latent history found` names the branch root, every other divergence names the exact event.
fn parse_error_location(error: &str) -> Option<Location> {
	let location = if let Some(rest) = error.strip_prefix("latent history found: ") {
		rest.split_once(" in root {")?.1.split_once("}: ")?.0
	} else {
		error.split_once(" at {")?.1.split_once('}')?.0
	};

	format!("{{{location}}}").parse().ok()
}

/// Why a workflow must not be repaired: it is running, about to run, silenced, or finished. A
/// repair that writes history under a live workflow would race its worker.
fn live_workflow_refusal(meta: &WorkflowMeta) -> Option<String> {
	if meta.has_lease || meta.has_worker {
		Some("workflow is leased by a worker".to_string())
	} else if meta.has_wake_condition {
		Some("workflow has a pending wake condition and is about to run".to_string())
	} else if meta.is_silenced {
		Some("workflow is silenced".to_string())
	} else if meta.is_complete {
		Some("workflow is complete".to_string())
	} else {
		None
	}
}

/// Recognizes an incomplete `set_error` activity written by a previous run of this repair.
fn is_inserted_set_error(event: &RawEvent, version: usize) -> bool {
	event.is_activity(SET_ERROR_ACTIVITY)
		&& event.version == Some(version)
		&& event.create_ts.is_some()
		&& event.input() == SET_ERROR_INPUT
}

/// Mirrors `history::cursor::Cursor::current_location_for` for `HistoryResult::Insertion`, which is
/// how gasoline itself picks a coordinate for an event inserted before an existing one. The
/// inserted event has to land exactly where the cursor would put it or replay diverges again.
fn insertion_coordinate(previous: Option<&Coordinate>, current: &Coordinate) -> Coordinate {
	// A coordinate of 0 is the left-most bound, matching `Cursor::new`.
	let fallback = Coordinate::simple(0);
	let previous = previous.unwrap_or(&fallback);

	match previous.len().cmp(&current.len()) {
		// 1.1 vs 1.1.1, prev + .0.1
		Ordering::Less => previous
			.iter()
			.cloned()
			.chain([0, 1])
			.collect::<Coordinate>(),
		// 1.1 vs 1.2, prev + .1
		Ordering::Equal => previous.iter().cloned().chain([1]).collect::<Coordinate>(),
		// 1.3.1 vs 1.4, increment the tail
		Ordering::Greater => previous.with_tail(previous.tail() + 1),
	}
}

/// Parses the location out of the one divergence message this repair handles.
fn parse_deallocate_divergence(error: &str) -> Option<Location> {
	let rest = error.strip_prefix("history diverged: ")?;
	let rest = rest.strip_prefix(&format!("expected activity {DEALLOCATE_ACTIVITY:?} at "))?;
	let (location, rest) = rest.split_once('}')?;

	if rest != format!(", found activity {SET_ERROR_ACTIVITY:?}") {
		return None;
	}

	format!("{location}}}").parse().ok()
}

/// Classifies a workflow that replayed past its repair. Gasoline persists the reason a workflow
/// went back to sleep in the error field, so a healthy sleeping workflow always has one.
fn classify_replayed_workflow(meta: &WorkflowMeta) -> RepairVerifyState {
	match meta.error.as_deref() {
		_ if meta.is_complete => RepairVerifyState::Recovered,
		None => RepairVerifyState::Recovered,
		Some(error)
			if error.starts_with("no signal found:")
				|| error.starts_with("sleeping until ")
				|| error == "workflow evicted" =>
		{
			RepairVerifyState::Recovered
		}
		Some(_) => RepairVerifyState::UnrelatedError,
	}
}
