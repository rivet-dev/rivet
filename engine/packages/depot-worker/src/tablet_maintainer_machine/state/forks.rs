//! Destination coordination is a bounded activity of the existing tablet owner.
//! Durable preparation outlives callers; network work never owns the store FIFO.
use super::{MaintainerPhase, MaintainerState, Operation, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{MaintainerTimer, OpId, RequestId},
};
use depot_fs_core::work::PendingVolume;
use depot_fs_core::{
	forks::{Preparation, StoreAction, StoreResult},
	*,
};
use depot_types::VolumeKey;
use rivet_data::generated::{depot_client_protocol_v1::*, shared_depot_fs_read_v1::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod restore;
pub(crate) use restore::quiescing;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Forks {
	pub jobs: BTreeMap<VolumeKey, Job>,
	/// Destinations whose pending fork discovery found while every job was taken. Bounded by
	/// `max_loaded_volumes`, as `fork_checked` is.
	pub(super) skipped: super::pending_work::Skipped,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Job {
	pub spec: FsForkSpec,
	phase: Phase,
	callers: BTreeSet<RequestId>,
	/// The maintainer log op that is the publication's durable record, once it committed.
	log: Option<super::maintainer_log::LogToken>,
	/// The job resumes a publication replayed from the log: its op exists, and the replay
	/// finishes when the job publishes again.
	replay: bool,
	/// The maintainer log op that fails a job whose publication was logged, once it
	/// committed. The job replies with the failure only after it, so a replay never
	/// publishes a fork the client was told failed.
	fail_log: Option<super::maintainer_log::LogToken>,
	/// A restore's ready image, kept from its drain until its publication is logged.
	restore_image: Option<FsForkImage>,
}

impl Job {
	/// A bounded diagnostic label; never expose a preparation's data or credentials.
	pub(crate) fn activity_phase(&self) -> &'static str {
		match self.phase {
			Phase::Queued(_) => "QUEUED",
			Phase::Storing(_) => "STORING",
			Phase::Ready(_) => "READY",
			Phase::Peer(_) => "PEER",
			Phase::Waiting(_) => "WAITING",
			Phase::Logging(_) => "LOGGING",
			Phase::LoggingFailure(_) => "LOGGING_FAILURE",
			Phase::FailureWaiting(_) => "FAILURE_WAITING",
		}
	}
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase {
	/// Owns the logical name before local persistence can begin.
	Queued(StoreAction),
	/// Only this operation's local completion can advance the job.
	Storing(LocalStep),
	/// Durable progress ready for the next bounded step.
	Ready(FsForkPreparation),
	/// Retains the exact image/cursor while its regional attempt runs.
	Peer(FsForkPreparation),
	/// Transport failure is uncertain; retry the same durable preparation.
	Waiting(FsForkPreparation),
	/// The publication's maintainer log op is committing; the preparation is ready.
	Logging(FsForkPreparation),
	/// The failure op of a job whose publication was logged is committing. Once it commits
	/// the job writes the failure and replies.
	LoggingFailure(FsForkError),
	/// The log was not serving when the failure op was due; the fork retry timer submits it
	/// again.
	FailureWaiting(FsForkError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum LocalStep {
	Inspect,
	Prepare,
	Remember,
	Copy,
	RestoreTarget,
	Publish,
	Fail,
}
fn step(action: &StoreAction) -> LocalStep {
	match action {
		StoreAction::Inspect => LocalStep::Inspect,
		StoreAction::Prepare => LocalStep::Prepare,
		StoreAction::Remember(_) => LocalStep::Remember,
		StoreAction::Copy { .. } => LocalStep::Copy,
		StoreAction::RestoreTarget => LocalStep::RestoreTarget,
		StoreAction::Publish { .. } => LocalStep::Publish,
		StoreAction::Fail(_) => LocalStep::Fail,
	}
}
pub(crate) fn pending_requests(state: &MaintainerState) -> usize {
	state.forks.jobs.values().map(|j| j.callers.len()).sum()
}
pub(crate) fn holds_key(state: &MaintainerState, key: &VolumeKey) -> bool {
	state.forks.jobs.contains_key(key)
}
fn respond(id: RequestId, response: ClientResponse, fx: &mut Effects) {
	fx.cancel_timer(MaintainerTimer::ForkRequest(id));
	fx.execute(MaintainerEffect::Reply {
		request_id: id,
		response,
	});
}
fn rejected(error: FsForkError) -> ClientResponse {
	ClientResponse::FsForkResponse(FsForkResponse::FsForkError(error))
}
/// The store holds the publication, so its live op is applied. A resumed replay's op is
/// finished when its job ends; see `finish`.
fn published(state: &mut MaintainerState, key: &VolumeKey, fx: &mut Effects) {
	let Some(job) = state.forks.jobs.get(key) else {
		return;
	};
	if let (Some(token), false) = (job.log, job.replay) {
		super::maintainer_log::applied(state, token, fx);
	}
}

fn finish(
	state: &mut MaintainerState,
	key: &VolumeKey,
	response: ClientResponse,
	fx: &mut Effects,
) {
	// A resumed replay ends with its job, published or not, so the destination's opens and
	// the tablet's sweeps stop waiting for it. A live job's logged op is applied however the
	// job ends, so a rejected publication does not hold back the log's trim.
	match state.forks.jobs.get(key) {
		Some(Job {
			log: Some(token),
			replay: true,
			..
		}) => {
			let token = *token;
			super::maintainer_log::replay::finished(state, token, fx);
		}
		Some(Job {
			log: Some(token),
			replay: false,
			..
		}) => {
			let token = *token;
			super::maintainer_log::applied(state, token, fx);
		}
		Some(Job { log: None, .. }) | None => {}
	}
	if let Some(token) = state.forks.jobs.get(key).and_then(|job| job.fail_log) {
		super::maintainer_log::applied(state, token, fx);
	}
	if let Some(job) = state.forks.jobs.remove(key) {
		fx.cancel_timer(MaintainerTimer::ForkRetry(key.clone()));
		for id in job.callers {
			respond(id, response.clone(), fx);
		}
		job_freed(state, fx);
	}
}

/// A job ended, so one destination whose pending fork discovery skipped for want of a job is
/// scanned again.
fn job_freed(state: &mut MaintainerState, fx: &mut Effects) {
	super::pending_work::Skipped::job_ended(
		state,
		|state| &mut state.forks.skipped,
		"fork_job",
		fx,
	);
}

/// Whether a discovered fork would get a job now.
pub(crate) fn has_free_job(state: &MaintainerState) -> bool {
	state.forks.jobs.len() < state.config.max_background_cold_reads as usize
}

/// Remembers that discovery found `key`'s pending fork while every job was taken.
fn skip(state: &mut MaintainerState, key: &VolumeKey) {
	let cap = state.config.max_loaded_volumes as usize;
	state.forks.skipped.skip(PendingVolume::of(key), cap);
}
fn queue(state: &mut MaintainerState, key: &VolumeKey, action: StoreAction) {
	state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::Queued(action);
	state.store_queue.push_back(StoreWork::Fork {
		volume_key: key.clone(),
	});
}
pub(crate) fn admit(
	state: &mut MaintainerState,
	id: RequestId,
	spec: FsForkSpec,
	deadline: u64,
	fx: &mut Effects,
) {
	let key = &spec.destination_volume_key;
	let routing = if !matches!(
		state.phase,
		MaintainerPhase::Serving | MaintainerPhase::Frozen
	) || !state
		.tablet
		.hash_range
		.contains(depot_types::partition_key_hash(key))
	{
		Some(depot_types::client::not_maintainer())
	} else if state.phase == MaintainerPhase::Frozen {
		Some(VolumeRejectResponse::TabletSplitting)
	} else if deadline <= fx.now_boot_ms()
		|| super::maintainer_log::replay::pending_for(state, key)
		|| super::maintainer_log::replay::pending_for(state, &spec.parent_volume_key)
		|| state.request_count() >= state.config.max_requests as usize
		|| key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
		|| spec.parent_volume_key.byte_len() > state.config.tuning().max_volume_key_bytes as usize
	{
		Some(depot_types::client::overloaded())
	} else {
		None
	};
	if let Some(error) = routing {
		respond(id, ClientResponse::VolumeRejectResponse(error), fx);
		return;
	}
	if !depot_fs_core::forks::valid_spec(&spec, state.config.local_region) {
		respond(id, rejected(FsForkError::Invalid), fx);
		return;
	}
	// A fork or restore over a name a repair claim holds would drain the claim's writer.
	if let Some(reason) = super::operator_repair::claims::under_repair(state, key) {
		respond(id, ClientResponse::VolumeRejectResponse(reason), fx);
		return;
	}
	// A fork or restore over a name whose current incarnation has no stored record would
	// have to drain and replace it, and a fork from one would read an image the store may
	// not hold; both wait for that incarnation's delete.
	if let Some(incarnation) = super::filesystem::record_missing(state, key)
		.or_else(|| super::filesystem::record_missing(state, &spec.parent_volume_key))
	{
		respond(
			id,
			ClientResponse::VolumeRejectResponse(depot_types::client::record_missing(incarnation)),
			fx,
		);
		return;
	}
	let now_boot_ms = state.now_boot_ms;
	for (_, volume) in state.loaded_volumes.range_mut(super::incarnations(key)) {
		volume.last_activity = now_boot_ms;
	}
	if let Some(job) = state.forks.jobs.get_mut(key) {
		if job.spec != spec {
			let error = if job.spec.operation_id == spec.operation_id {
				FsForkError::OperationConflict
			} else {
				FsForkError::Busy
			};
			respond(id, rejected(error), fx);
			return;
		}
		job.callers.insert(id);
	} else {
		if state.forks.jobs.len() >= state.config.max_background_cold_reads as usize {
			respond(
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
		state.fork_checked.remove(key);
		state.forks.skipped.started(&PendingVolume::of(key));
		state.store_queue.push_back(StoreWork::Fork {
			volume_key: key.clone(),
		});
		state.forks.jobs.insert(
			key.clone(),
			Job {
				spec,
				phase: Phase::Queued(StoreAction::Inspect),
				callers: BTreeSet::from([id]),
				log: None,
				replay: false,
				fail_log: None,
				restore_image: None,
			},
		);
	}
	fx.arm_timer_at(MaintainerTimer::ForkRequest(id), deadline);
}
/// The bounded pending prefix scan supplies identities, never a stale phase. A fork found
/// while every job is taken stays durable and is scanned again when a job ends.
pub(crate) fn discover(
	state: &mut MaintainerState,
	spec: FsForkSpec,
) -> std::result::Result<(), ()> {
	let key = &spec.destination_volume_key;
	if !depot_fs_core::forks::valid_spec(&spec, state.config.local_region)
		|| !state
			.tablet
			.hash_range
			.contains(depot_types::partition_key_hash(key))
	{
		return Err(());
	}
	if state.phase != MaintainerPhase::Serving || state.forks.jobs.contains_key(key) {
		return Ok(());
	}
	if !has_free_job(state) {
		skip(state, key);
		return Ok(());
	}
	state.fork_checked.remove(key);
	state.forks.skipped.started(&PendingVolume::of(key));
	state.store_queue.push_back(StoreWork::Fork {
		volume_key: key.clone(),
	});
	state.forks.jobs.insert(
		key.clone(),
		Job {
			spec,
			phase: Phase::Queued(StoreAction::Inspect),
			callers: BTreeSet::new(),
			log: None,
			replay: false,
			fail_log: None,
			restore_image: None,
		},
	);
	Ok(())
}

/// A replayed publication found its pages may be lost, and the store holds the fork's
/// preparation again. A job copies the pages again and publishes under the replayed op,
/// whose pending replay keeps the destination's opens and reads waiting meanwhile.
pub(crate) fn resume(
	state: &mut MaintainerState,
	spec: FsForkSpec,
	token: super::maintainer_log::LogToken,
) {
	let key = spec.destination_volume_key.clone();
	state.fork_checked.remove(&key);
	state.store_queue.push_back(StoreWork::Fork {
		volume_key: key.clone(),
	});
	state.forks.jobs.insert(
		key,
		Job {
			spec,
			phase: Phase::Queued(StoreAction::Inspect),
			callers: BTreeSet::new(),
			log: Some(token),
			replay: true,
			fail_log: None,
			restore_image: None,
		},
	);
}

/// The source incarnation whose image `job`'s preparation copies or holds, once the acquire
/// named one.
fn source_image_key(job: &Job) -> Option<depot_types::VolumeLogKey> {
	let preparation = match &job.phase {
		Phase::Ready(preparation)
		| Phase::Peer(preparation)
		| Phase::Waiting(preparation)
		| Phase::Logging(preparation) => preparation,
		Phase::Queued(_)
		| Phase::Storing(_)
		| Phase::LoggingFailure(_)
		| Phase::FailureWaiting(_) => return None,
	};
	let image = match preparation {
		FsForkPreparation::FsForkCopying(copy) => &copy.image,
		FsForkPreparation::FsForkReady(ready) => &ready.image,
		FsForkPreparation::FsForkAcquirePending => return None,
	};
	Some(depot_types::VolumeLogKey(
		rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: image.link.volume_key.clone(),
			volume_incarnation: image.link.incarnation,
		},
	))
}

/// Fails `key`'s fork. A live job whose publication was logged first logs the failure, so a replay
/// after a crash fails the fork again instead of publishing it.
///
/// A job resuming a replayed publication never fails it: the live job logged the publication
/// only once its copy had succeeded and its client may hold the acknowledgement, and the
/// source keeps its hold on the image until a pass covers the copied pages, so a copy or
/// publish that fails now means this maintainer's store or replay disagrees with the log.
/// The logged op stays for a new owner to replay against the store it finds.
fn fail(state: &mut MaintainerState, key: &VolumeKey, error: FsForkError, fx: &mut Effects) {
	let job = &state.forks.jobs[key];
	if job.replay {
		let image_unreadable = matches!(
			error,
			FsForkError::BaseUnavailable
				| FsForkError::CheckpointNotFound
				| FsForkError::VolumeNotFound
		);
		let source_lost = source_image_key(job)
			.filter(|source| super::filesystem::is_record_missing(state, source));
		if let (Some(source), true, Phase::Peer(preparation)) =
			(source_lost, image_unreadable, &job.phase)
		{
			crate::metrics::TABLET_FORK_REPLAY_RECORD_MISSING.inc();
			tracing::debug!(
				?key,
				?error,
				source = ?source,
				"an acknowledged fork waits for operator repair of its image source record"
			);
			wait(state, key, preparation.clone(), fx);
			return;
		}
		tracing::warn!(
			?key,
			?error,
			"a resumed publication replay cannot copy or publish its image; fencing so a new owner replays it"
		);
		super::fence(state, fx);
		return;
	}
	if job.log.is_none() || job.fail_log.is_some() {
		queue(state, key, StoreAction::Fail(error));
		return;
	}
	// A failure op ignores the log's byte cap: the publication it fails holds back trim until
	// the job ends, so refusing it on a full log would keep the log full for good. At most
	// one failure op exists per fork job, so the overshoot stays bounded.
	if !state.maintainer_log.serving() {
		state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::FailureWaiting(error);
		fx.arm_timer_after(
			MaintainerTimer::ForkRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
		return;
	}
	let op = rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkFailOp(
		rivet_data::generated::depot_maintainer_log_v1::ForkFailOp {
			spec: job.spec.clone(),
			error: error.clone(),
		},
	);
	state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::LoggingFailure(error);
	super::maintainer_log::submit(
		state,
		op,
		None,
		super::maintainer_log::Waiter::ForkFailed(key.clone()),
	);
}

/// The failure op of `key`'s fork committed at `token`: write the failure.
pub(crate) fn fail_logged(
	state: &mut MaintainerState,
	key: VolumeKey,
	token: super::maintainer_log::LogToken,
	fx: &mut Effects,
) {
	let Some(job) = state.forks.jobs.get_mut(&key) else {
		super::maintainer_log::applied(state, token, fx);
		return;
	};
	let Phase::LoggingFailure(error) = job.phase.clone() else {
		super::maintainer_log::applied(state, token, fx);
		return;
	};
	job.fail_log = Some(token);
	queue(state, &key, StoreAction::Fail(error));
}

/// The publication op of `key`'s fork committed at `token`: publish.
pub(crate) fn logged(
	state: &mut MaintainerState,
	key: VolumeKey,
	token: super::maintainer_log::LogToken,
	fx: &mut Effects,
) {
	// A job that ended while its op was committing publishes nothing from it, so the op is
	// applied and the log may trim past it.
	let Some(job) = state.forks.jobs.get_mut(&key) else {
		super::maintainer_log::applied(state, token, fx);
		return;
	};
	if !matches!(job.phase, Phase::Logging(_)) {
		super::maintainer_log::applied(state, token, fx);
		return;
	}
	job.log = Some(token);
	// A restore's op was logged after its drain claimed the incarnation it replaces.
	let claimed = job.spec.restore;
	let Some(incarnation_epoch) = logged_epoch(state, token) else {
		super::fence(state, fx);
		return;
	};
	queue(
		state,
		&key,
		StoreAction::Publish {
			claimed,
			logged: true,
			incarnation_epoch,
		},
	);
}

/// The ordering epoch the publication op at `token` gave its incarnation.
fn logged_epoch(state: &MaintainerState, token: super::maintainer_log::LogToken) -> Option<u64> {
	match super::maintainer_log::op(state, token)? {
		rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkPublishOp(op) => {
			Some(op.incarnation_epoch)
		}
		_ => None,
	}
}

/// Draws the ordering epoch of a publication of `key` that is not logged yet: above every
/// incarnation of the name this maintainer knows and above `floor`, the replaced incarnation's
/// epoch for a restore. `None` when this lifetime can issue no such epoch.
fn draw_epoch(state: &mut MaintainerState, key: &VolumeKey, floor: u64) -> Option<u64> {
	let floor = floor.max(super::replica_creation::highest_epoch(state, key));
	super::claim_epochs::issue(state, floor.checked_add(1)?)
}

/// The ordering epoch a publication of `key` publishes under: its logged op's when the job
/// resumes a replayed publication, else a newly drawn one.
fn publication_epoch(state: &mut MaintainerState, key: &VolumeKey, floor: u64) -> Option<u64> {
	match state.forks.jobs.get(key).and_then(|job| job.log) {
		Some(token) => logged_epoch(state, token),
		None => draw_epoch(state, key, floor),
	}
}
/// Inspect receipts first. Only a genuinely new preparation depends on current
/// RAM-only creations and outstanding lifecycle work.
fn available(state: &MaintainerState, spec: &FsForkSpec) -> std::result::Result<(), FsForkError> {
	let key = &spec.destination_volume_key;
	if state.pending.contains_key(key)
		|| super::deletion::holds_key(state, key)
		|| super::repair::holds_key(state, key)
		|| super::catch_up::holds_key(state, key)
		|| state
			.unloading
			.range(super::incarnations(key))
			.next()
			.is_some()
		|| state.operations.values().any(|op| match op {
			Operation::Load { key: k, .. } => k.0.volume_key == *key,
			Operation::ReadRow { volume_key: k } => k == key,
			_ => false,
		}) {
		return Err(FsForkError::Busy);
	}
	let current: Vec<_> = state
		.loaded_volumes
		.range(super::incarnations(key))
		.filter(|(k, _)| !super::deletion::is_replaced(state, k))
		.collect();
	if current.len() > 1 {
		return Err(FsForkError::Busy);
	}
	if let Some((key, volume)) = current.first() {
		if volume.creation.as_ref().is_some_and(|v| !v.acknowledged) {
			return Err(FsForkError::Busy);
		}
		match spec.restore {
			None => return Err(FsForkError::DestinationExists),
			Some(expected) if expected != key.0.volume_incarnation || volume.is_deleted() => {
				return Err(FsForkError::IncarnationChanged);
			}
			// A restore of a volume whose first record has not reached the store checks
			// the record registered for it instead; see `unrecorded_replaced`.
			Some(_)
				if volume.stored_row.is_none() && unrecorded_replaced(state, spec).is_none() =>
			{
				return Err(FsForkError::Busy);
			}
			Some(_) => {}
		}
	}
	Ok(())
}

/// The record a restore replaces when that incarnation's first record has not reached the
/// store: the record its apply registered, which the ordinary pass will write. The restore
/// checks and archives it as the store's own, so it asks for no pass.
fn unrecorded_replaced(state: &MaintainerState, spec: &FsForkSpec) -> Option<FsVolumeRecord> {
	let replaced = spec.restore?;
	let key =
		depot_types::VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: spec.destination_volume_key.clone(),
			volume_incarnation: replaced,
		});
	let loaded = state.loaded_volumes.get(&key)?;
	if loaded.stored_row.is_some() {
		return None;
	}
	let record = state.filesystem.volumes.get(&key)?.record.to_record();
	(record.status == FsVolumeStatus::Live).then_some(record)
}

pub(crate) fn dispatch(state: &mut MaintainerState, key: VolumeKey, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let Some(job) = state.forks.jobs.get(&key) else {
		return;
	};
	let action = match &job.phase {
		Phase::Queued(action) => action.clone(),
		_ => return,
	};
	let spec = job.spec.clone();
	if matches!(action, StoreAction::RestoreTarget) && !restore::quiet(state, &key) {
		// Existing replica work retains its ownership until completion. The drive
		// pass requeues this local step once that work has drained.
		return;
	}
	// Listings and already-admitted opens may have completed since Inspect.
	if matches!(action, StoreAction::Prepare)
		&& let Err(error) = available(state, &spec)
	{
		finish(state, &key, rejected(error), fx);
		return;
	}
	let replaced = unrecorded_replaced(state, &spec);
	state.forks.jobs.get_mut(&key).unwrap().phase = Phase::Storing(step(&action));
	let parent_volume_key =
		if super::in_tablet(state, &spec.parent_volume_key) && spec.parent_volume_key != key {
			Some(spec.parent_volume_key.clone())
		} else {
			None
		};
	let op_id = state.begin(
		Operation::ForkStore {
			volume_key: key,
			parent_volume_key,
		},
		fx,
	);
	fx.execute(MaintainerEffect::ForkStoreRequest {
		op_id,
		spec,
		region: state.config.local_region,
		action,
		replaced,
	});
}
pub(crate) fn stored(
	state: &mut MaintainerState,
	id: OpId,
	result: std::result::Result<StoreResult, String>,
	fx: &mut Effects,
) {
	if !matches!(state.operations.get(&id), Some(Operation::ForkStore { .. })) {
		return;
	}
	let Some(Operation::ForkStore {
		volume_key: key, ..
	}) = state.finish(id, fx)
	else {
		unreachable!()
	};
	let Some(job) = state.forks.jobs.get(&key) else {
		super::fence(state, fx);
		return;
	};
	let Phase::Storing(step) = job.phase else {
		super::fence(state, fx);
		return;
	};
	let spec = job.spec.clone();
	match result {
		Err(_) => super::fence(state, fx),
		Ok(StoreResult::RestoreTarget(record))
			if step == LocalStep::RestoreTarget
				&& record.volume_key() == key
				&& Some(record.volume_incarnation) == spec.restore
				&& record.status == FsVolumeStatus::Live =>
		{
			log_restore(state, &key, &record, fx);
		}
		Ok(StoreResult::RestoreTarget(_)) => super::fence(state, fx),
		Ok(StoreResult::Unprepared) if step == LocalStep::Inspect => {
			match available(state, &spec) {
				Ok(()) => queue(state, &key, StoreAction::Prepare),
				Err(error) => finish(state, &key, rejected(error), fx),
			}
		}
		Ok(StoreResult::Unprepared) => super::fence(state, fx),
		Ok(StoreResult::Progress(Preparation::Pending(pending))) => {
			if pending.spec != spec {
				super::fence(state, fx);
				return;
			}
			state.forks.jobs.get_mut(&key).unwrap().phase = Phase::Ready(pending.preparation);
		}
		Ok(StoreResult::Progress(Preparation::Complete(record))) => {
			if record.volume_key() != key || record.volume_incarnation != spec.operation_id {
				super::fence(state, fx);
				return;
			}
			super::alarms::published(state, &record, fx);
			if let Some(old) = spec.restore {
				let old = depot_types::VolumeLogKey(
					rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
						volume_key: key.clone(),
						volume_incarnation: old,
					},
				);
				super::deletion::retire(state, old, fx);
			}
			published(state, &key, fx);
			finish(
				state,
				&key,
				ClientResponse::FsForkResponse(FsForkResponse::FsVolumeRecord(record)),
				fx,
			);
		}
		Ok(StoreResult::Rejected(_)) if step == LocalStep::Publish && spec.restore.is_some() => {
			// The job holds the name and checked the replaced record before logging, so a
			// refusal here means the store disagrees with the core. The logged op stays for a
			// new owner to replay against the store it finds.
			super::fence(state, fx);
		}
		Ok(StoreResult::Rejected(error)) => {
			if matches!(
				step,
				LocalStep::Inspect | LocalStep::Prepare | LocalStep::Fail
			) {
				// A resumed publication the store refuses before any copy has moved on
				// without it, as when its destination was recreated or its replaced
				// incarnation is gone; the op is done, as before this lifetime.
				if state.forks.jobs[&key].replay {
					tracing::warn!(
						?key,
						?error,
						"the store refused a resumed publication replay before its copy; the op is done"
					);
				}
				finish(state, &key, rejected(error), fx);
			} else {
				fail(state, &key, error, fx);
			}
		}
	}
}
/// Whether `link` is the image a fork or restore copies while it resumes a publication
/// replayed from the maintainer log, and no recovered op of the image's name logged before
/// that publication is still to replay. The pending op such a read would wait for is the
/// job's own publication, which cannot apply before the copy. A restore's image belongs to an
/// older incarnation of its own name; a fork's to its parent.
pub(crate) fn resumes_replayed_fork(state: &MaintainerState, link: &FsBaseLink) -> bool {
	let earliest = super::maintainer_log::replay::earliest_pending_token(state, &link.volume_key);
	state.forks.jobs.values().any(|job| {
		let Some(token) = job.log else {
			return false;
		};
		if !job.replay || earliest.is_some_and(|earliest| earliest < token) {
			return false;
		}
		let preparation = match &job.phase {
			Phase::Ready(preparation) | Phase::Peer(preparation) | Phase::Waiting(preparation) => {
				preparation
			}
			Phase::Queued(_)
			| Phase::Storing(_)
			| Phase::Logging(_)
			| Phase::LoggingFailure(_)
			| Phase::FailureWaiting(_) => return false,
		};
		let image = match preparation {
			FsForkPreparation::FsForkCopying(copy) => &copy.image,
			FsForkPreparation::FsForkReady(ready) => &ready.image,
			FsForkPreparation::FsForkAcquirePending => return false,
		};
		image.link == *link
	})
}

/// Whether a restore job replaces `key`, the incarnation it publishes over.
pub(crate) fn restores(forks: &Forks, key: &depot_types::VolumeLogKey) -> bool {
	forks
		.jobs
		.get(&key.0.volume_key)
		.is_some_and(|job| job.spec.restore == Some(key.0.volume_incarnation))
}

pub(crate) fn drive(state: &mut MaintainerState, fx: &mut Effects) {
	if state.phase != MaintainerPhase::Serving {
		return;
	}
	let ready: Vec<_> = state
		.forks
		.jobs
		.iter()
		.filter_map(|(key, job)| {
			if let Phase::Ready(preparation) = &job.phase {
				Some((key.clone(), job.spec.clone(), preparation.clone()))
			} else {
				None
			}
		})
		.collect();
	for (key, spec, preparation) in ready {
		let request = match &preparation {
			FsForkPreparation::FsForkAcquirePending => ClientRequest::FsForkReferenceRequest(
				FsForkReferenceRequest::FsForkAcquire(FsForkAcquire { spec: spec.clone() }),
			),
			FsForkPreparation::FsForkCopying(copy) => ClientRequest::FsReadRequest(FsReadRequest {
				target: FsReadTarget::VolumeLogKey(
					rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
						volume_key: copy.image.link.volume_key.clone(),
						volume_incarnation: copy.image.link.incarnation,
					},
				),
				client_epoch: None,
				operation: FsReadOperation::FsForkMetadataRequest(FsForkMetadataRequest {
					link: copy.image.link.clone(),
					after: copy.cursor.clone(),
					limit: state.config.tuning().max_rows_per_pass,
				}),
			}),
			FsForkPreparation::FsForkReady(_) => {
				if let Some(old) = spec.restore {
					// Apply the old incarnation's final tail before its apply work stops.
					let old = depot_types::VolumeLogKey(
						rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
							volume_key: key.clone(),
							volume_incarnation: old,
						},
					);
					// An old incarnation this tablet has not seen may still have replicas while
					// the listing is incomplete or the replica set is changing; wait for it.
					if !state.loaded_volumes.contains_key(&old)
						&& !super::opening::orphans_provable(state)
					{
						continue;
					}
					if state.loaded_volumes.contains_key(&old)
						&& !super::drain::finished(state, &old)
					{
						if !state.drains.contains_key(&old) {
							// No claim epoch to issue: the next drive asks again.
							let Some(client) =
								super::drain::fencing_client(state, &old, 0, spec.operation_id)
							else {
								continue;
							};
							super::drain::start(state, &old, client, None);
						}
						continue;
					}
					// The finished drain stays until the publication is logged, so a retry
					// checks the target again without claiming again.
					if let FsForkPreparation::FsForkReady(ready) = &preparation {
						state.forks.jobs.get_mut(&key).unwrap().restore_image =
							Some(ready.image.clone());
					}
					queue(state, &key, StoreAction::RestoreTarget);
					restore::quiesce(state, &key, fx);
				} else {
					publish(state, &key, &spec, &preparation, fx);
				}
				continue;
			}
		};
		state.forks.jobs.get_mut(&key).unwrap().phase = Phase::Peer(preparation);
		let op_id = state.begin(Operation::ForkPeer { volume_key: key }, fx);
		fx.execute(MaintainerEffect::ForkPeerRequest {
			op_id,
			region: spec.parent_region,
			request,
			deadline_boot_ms: fx
				.now_boot_ms()
				.saturating_add(state.config.tuning().operation_timeout_ms),
		});
	}
	restore::drive(state);
}
/// Publishes a ready fork that is not a restore; `log_restore` logs a restore's. Its durable
/// record is a maintainer log op, so the publication waits for that op to commit and then
/// writes nothing to cold. A job that resumes a replayed publication already has its op.
fn publish(
	state: &mut MaintainerState,
	key: &VolumeKey,
	spec: &FsForkSpec,
	preparation: &FsForkPreparation,
	fx: &mut Effects,
) {
	let replay = state.forks.jobs[key].replay;
	if replay {
		let Some(incarnation_epoch) = publication_epoch(state, key, 0) else {
			wait(state, key, preparation.clone(), fx);
			return;
		};
		queue(
			state,
			key,
			StoreAction::Publish {
				claimed: None,
				logged: replay,
				incarnation_epoch,
			},
		);
		return;
	}
	let FsForkPreparation::FsForkReady(ready) = preparation else {
		return;
	};
	if !super::maintainer_log::admits(state) {
		wait(state, key, preparation.clone(), fx);
		return;
	}
	let Some(incarnation_epoch) = draw_epoch(state, key, 0) else {
		wait(state, key, preparation.clone(), fx);
		return;
	};
	let op = rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkPublishOp(
		rivet_data::generated::depot_maintainer_log_v1::ForkPublishOp {
			spec: spec.clone(),
			image: ready.image.clone(),
			target: None,
			incarnation_epoch,
		},
	);
	state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::Logging(preparation.clone());
	super::maintainer_log::submit(
		state,
		op,
		None,
		super::maintainer_log::Waiter::Fork(key.clone()),
	);
}

/// Logs a restore's publication once its drain claimed the replaced incarnation's replicas and the
/// store still names that incarnation live. The op names the replaced incarnation, at the image
/// position when the image is that incarnation's own checkpoint, so a replay waits for its apply to
/// reach the image again. A job resuming a replayed publication already has its op.
fn log_restore(
	state: &mut MaintainerState,
	key: &VolumeKey,
	replaced: &FsVolumeRecord,
	fx: &mut Effects,
) {
	let job = &state.forks.jobs[key];
	let claimed = job.spec.restore;
	let spec = job.spec.clone();
	let restore_image = job.restore_image.clone();
	if job.replay {
		let logged = job.replay;
		let Some(incarnation_epoch) = publication_epoch(state, key, replaced.incarnation_epoch)
		else {
			super::fence(state, fx);
			return;
		};
		forget_drain(state, key, fx);
		queue(
			state,
			key,
			StoreAction::Publish {
				claimed,
				logged,
				incarnation_epoch,
			},
		);
		return;
	}
	let Some(image) = restore_image else {
		super::fence(state, fx);
		return;
	};
	let preparation = FsForkPreparation::FsForkReady(FsForkReady {
		image: image.clone(),
	});
	if !super::maintainer_log::admits(state) {
		// The drive drains and checks the target again once the retry timer fires.
		wait(state, key, preparation, fx);
		return;
	}
	let Some(incarnation_epoch) = draw_epoch(state, key, replaced.incarnation_epoch) else {
		wait(state, key, preparation, fx);
		return;
	};
	// A replay must find the replaced incarnation registered, to publish over its record when
	// that record never reached the store, and applied through the image when the image is
	// its own checkpoint.
	let own_checkpoint = image.link.volume_key == *key
		&& image.link.incarnation == replaced.volume_incarnation
		&& image
			.link
			.region
			.is_none_or(|region| region == state.config.local_region);
	let after = if own_checkpoint {
		image.link.position.clone()
	} else {
		replaced.cold.clone()
	};
	let target = Some(
		rivet_data::generated::depot_maintainer_log_v1::MaintainerOpTarget {
			volume_log_key: rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: replaced.volume_incarnation,
			},
			after,
		},
	);
	let op = rivet_data::generated::depot_maintainer_log_v1::MaintainerOp::ForkPublishOp(
		rivet_data::generated::depot_maintainer_log_v1::ForkPublishOp {
			spec,
			image,
			target,
			incarnation_epoch,
		},
	);
	state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::Logging(preparation);
	super::maintainer_log::submit(
		state,
		op,
		None,
		super::maintainer_log::Waiter::Fork(key.clone()),
	);
	forget_drain(state, key, fx);
}

/// Drops a restore's finished drain of the incarnation it replaces.
fn forget_drain(state: &mut MaintainerState, key: &VolumeKey, fx: &mut Effects) {
	let Some(replaced) = state.forks.jobs.get(key).and_then(|job| job.spec.restore) else {
		return;
	};
	let replaced =
		depot_types::VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: key.clone(),
			volume_incarnation: replaced,
		});
	super::drain::forget(state, &replaced, fx);
}

fn wait(
	state: &mut MaintainerState,
	key: &VolumeKey,
	preparation: FsForkPreparation,
	fx: &mut Effects,
) {
	state.forks.jobs.get_mut(key).unwrap().phase = Phase::Waiting(preparation);
	fx.arm_timer_after(
		MaintainerTimer::ForkRetry(key.clone()),
		state.config.tuning().retry_ms,
	);
}
pub(crate) fn peer(
	state: &mut MaintainerState,
	id: OpId,
	result: std::result::Result<ClientResponse, String>,
	fx: &mut Effects,
) {
	if !matches!(state.operations.get(&id), Some(Operation::ForkPeer { .. })) {
		return;
	}
	let Some(Operation::ForkPeer { volume_key: key }) = state.finish(id, fx) else {
		unreachable!()
	};
	let Some(job) = state.forks.jobs.get(&key) else {
		super::fence(state, fx);
		return;
	};
	let Phase::Peer(preparation) = job.phase.clone() else {
		super::fence(state, fx);
		return;
	};
	match (result, &preparation) {
		(
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkReceipt(
				receipt,
			))),
			FsForkPreparation::FsForkAcquirePending,
		) if receipt.spec == job.spec => queue(state, &key, StoreAction::Remember(receipt.image)),
		(
			Ok(ClientResponse::FsReadResponse(FsReadResponse::FsForkMetadataPage(page))),
			FsForkPreparation::FsForkCopying(copy),
		) => queue(
			state,
			&key,
			StoreAction::Copy {
				after: copy.cursor.clone().map(Into::into),
				page,
			},
		),
		(
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkError(
				error,
			))),
			_,
		) if !matches!(
			error,
			FsForkError::Busy
				| FsForkError::Lagging
				| FsForkError::BaseUnavailable
				| FsForkError::RegionUnavailable
		) =>
		{
			fail(state, &key, error, fx)
		}
		(Ok(ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeNotFound)), _) => {
			fail(state, &key, FsForkError::VolumeNotFound, fx)
		}
		(Ok(ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeDeleted)), _) => {
			fail(state, &key, FsForkError::VolumeDeleted, fx)
		}
		// A replayed publication may already be acknowledged. Keep it pending while an
		// operator repairs the source, even when the source is on another worker.
		(
			Ok(ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeRecordMissing(
				missing,
			))),
			_,
		) if job.replay
			&& source_image_key(job).is_some_and(|source| {
				source.0.volume_incarnation == missing.volume_incarnation
			}) =>
		{
			crate::metrics::TABLET_FORK_REPLAY_RECORD_MISSING.inc();
			wait(state, &key, preparation, fx);
		}
		// A live job can fail before its publication is acknowledged.
		(
			Ok(ClientResponse::VolumeRejectResponse(VolumeRejectResponse::VolumeRecordMissing(_))),
			_,
		) => fail(state, &key, FsForkError::BaseUnavailable, fx),
		(
			Ok(ClientResponse::FsReadResponse(FsReadResponse::FsReadError(FsReadError::NotFound))),
			_,
		) => fail(state, &key, FsForkError::CheckpointNotFound, fx),
		(
			Ok(ClientResponse::FsReadResponse(FsReadResponse::FsReadError(
				FsReadError::Corrupt | FsReadError::TooLarge,
			))),
			_,
		) => fail(state, &key, FsForkError::Invalid, fx),
		// Lost replies and temporary routing failures leave the exact durable image
		// unchanged. A sent Acquire may already have created its source hold.
		_ => wait(state, &key, preparation, fx),
	}
}
pub(crate) fn retry(state: &mut MaintainerState, key: VolumeKey, fx: &mut Effects) {
	if let Some(job) = state.forks.jobs.get_mut(&key)
		&& let Phase::Waiting(preparation) = &job.phase
	{
		job.phase = Phase::Ready(preparation.clone());
	}
	if let Some(Job {
		phase: Phase::FailureWaiting(error),
		..
	}) = state.forks.jobs.get(&key)
	{
		let error = error.clone();
		fail(state, &key, error, fx);
	}
}
pub(crate) fn expire(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	for job in state.forks.jobs.values_mut() {
		if job.callers.remove(&id) {
			respond(
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
	}
}
pub(crate) fn fence(state: &mut MaintainerState, fx: &mut Effects) {
	state.forks.skipped.clear();
	for (key, job) in std::mem::take(&mut state.forks.jobs) {
		fx.cancel_timer(MaintainerTimer::ForkRetry(key));
		for id in job.callers {
			respond(
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::not_maintainer()),
				fx,
			);
		}
	}
}


