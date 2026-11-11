//! Destination coordination is a bounded activity of the existing tablet owner.
//! Durable preparation outlives callers; network work never owns the store FIFO.
use super::{MaintainerPhase, MaintainerState, Operation, filesystem::ApplyHold, store::StoreWork};
use crate::tablet_maintainer_machine::{
	Effects,
	effect::MaintainerEffect,
	types::{FORK_PIN_ROW_VERSION, MaintainerTimer, OpId, RequestId},
};
use depot_fs_core::work::PendingVolume;
use depot_fs_core::{
	forks::{Preparation, StoreAction, StoreResult},
	*,
};
use depot_types::VolumeKey;
use rivet_data::generated::{depot_client_protocol_v2::*, shared_depot_fs_read_v2::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod restore;
pub(crate) use restore::quiescing;

/// Volumes whose lazy fork walked the whole lineage and found them absent at the fork, so the
/// next open creates them as today instead of walking again. Taken only by that volume's
/// create. Bounded per tablet maintainer by `max_absent_lazy_forks`: past it the oldest mark is
/// forgotten, and that volume's next open runs a lazy fork job again. The job walks the lineage,
/// adds a store round and a caller round trip, and finds the volume absent again; when no job
/// is free or the tablet's entry version is not ready, the open is refused as overloaded and
/// the caller retries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AbsentLazyForks {
	/// Each marked volume's place in `by_order`.
	by_key: BTreeMap<VolumeKey, u64>,
	/// The marked volumes, oldest mark first.
	by_order: BTreeMap<u64, VolumeKey>,
	/// The place the next mark takes in `by_order`. It only grows, so two states holding the
	/// same marks after different histories compare unequal.
	next_order: u64,
}

impl AbsentLazyForks {
	pub(crate) fn contains(&self, key: &VolumeKey) -> bool {
		self.by_key.contains_key(key)
	}

	pub(crate) fn len(&self) -> usize {
		self.by_key.len()
	}

	/// Marks `key` absent as the newest mark, then forgets the oldest marks past `capacity`.
	/// Returns how many marks were forgotten.
	pub(crate) fn insert(&mut self, key: VolumeKey, capacity: usize) -> usize {
		let order = self.next_order;
		self.next_order += 1;
		if let Some(previous) = self.by_key.insert(key.clone(), order) {
			self.by_order.remove(&previous);
		}
		self.by_order.insert(order, key);

		let mut forgotten = 0;
		while self.by_key.len() > capacity {
			let Some((_, oldest)) = self.by_order.pop_first() else {
				break;
			};
			self.by_key.remove(&oldest);
			forgotten += 1;
		}
		forgotten
	}

	pub(crate) fn remove(&mut self, key: &VolumeKey) {
		if let Some(order) = self.by_key.remove(key) {
			self.by_order.remove(&order);
		}
	}

	/// Checks the two maps hold as many marks as each other and no more than `capacity`. It runs
	/// after every transition, so it stays constant-time; [`Self::validate_orders`] checks that
	/// the maps agree mark by mark.
	pub(crate) fn validate(&self, capacity: usize) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.by_key.len() == self.by_order.len(),
			"absent lazy fork marks drifted"
		);
		anyhow::ensure!(
			self.by_key.len() <= capacity,
			"absent lazy fork capacity exceeded"
		);
		Ok(())
	}
}

/// Marks `key` absent at its lazy fork, forgetting the oldest marks past the configured cap.
fn mark_absent(state: &mut MaintainerState, key: &VolumeKey) {
	let capacity = state.config.max_absent_lazy_forks as usize;
	let forgotten = state.lazy_forks_absent.insert(key.clone(), capacity);
	if forgotten > 0 {
		crate::metrics::GROUP_LAZY_FORKS_ABSENT_FORGOTTEN.inc_by(forgotten as u64);
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Forks {
	/// The running fork jobs, which the passes after every event visit. A parked job is in
	/// `parked` instead, so those passes never visit the parked jobs of idle volumes.
	pub jobs: BTreeMap<VolumeKey, Job>,
	/// Destinations of forks other than lazy ones whose pending fork discovery found while
	/// every job of their window, `max_background_cold_reads`, was taken. Bounded by
	/// `max_loaded_volumes`, as `fork_checked` is.
	pub(super) skipped: super::pending_work::Skipped,
	/// Destinations of lazy forks whose pending fork discovery found while every job of the
	/// lazy fork window, `max_lazy_fork_jobs`, was taken. Kept apart from `skipped`, so a
	/// destination is handed back only when its own window has room: handing one back while
	/// only the other window had room skips it again at once, and discovery, finding a free
	/// job, hands it back again without end. Bounded by `max_loaded_volumes`.
	pub(super) skipped_lazy: super::pending_work::Skipped,
	/// The jobs in `Phase::Parked`, oldest first, bounded by `max_parked_lazy_fork_jobs`.
	pub(super) parked: ParkedJobs,
}

impl Forks {
	/// The job holding `key`, running or parked.
	pub(crate) fn job(&self, key: &VolumeKey) -> Option<&Job> {
		self.jobs.get(key).or_else(|| self.parked.get(key))
	}
}

/// The parked lazy fork jobs, kept apart from `Forks::jobs` so that `drive`, the snapshot's job
/// counts and the request count, which run after every event, visit only running jobs. They are
/// held in the order they parked, so the cap evicts the oldest, and their callers are counted, so
/// every count costs nothing. A job is here exactly while it is `Phase::Parked`, and never also in
/// `Forks::jobs`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ParkedJobs {
	/// Each parked job, by destination.
	by_key: BTreeMap<VolumeKey, ParkedJob>,
	/// The parked destinations, oldest first.
	by_order: BTreeMap<u64, VolumeKey>,
	/// The place the next parked job takes in `by_order`. It only grows.
	next_order: u64,
	/// The callers of every parked job: fork requests of the same spec that joined one.
	callers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ParkedJob {
	/// The job's place in `ParkedJobs::by_order`.
	order: u64,
	job: Job,
}

impl ParkedJobs {
	pub(crate) fn len(&self) -> usize {
		self.by_key.len()
	}

	fn get(&self, key: &VolumeKey) -> Option<&Job> {
		self.by_key.get(key).map(|parked| &parked.job)
	}

	/// Parks `job` as the newest parked job.
	fn insert(&mut self, key: VolumeKey, job: Job) {
		let order = self.next_order;
		self.next_order += 1;
		self.callers += job.callers.len();
		if let Some(previous) = self.by_key.insert(key.clone(), ParkedJob { order, job }) {
			self.by_order.remove(&previous.order);
			self.callers -= previous.job.callers.len();
		}
		self.by_order.insert(order, key);
	}

	fn remove(&mut self, key: &VolumeKey) -> Option<Job> {
		let parked = self.by_key.remove(key)?;
		self.by_order.remove(&parked.order);
		self.callers -= parked.job.callers.len();
		Some(parked.job)
	}

	/// Adds fork request `id` to the callers of parked job `key`.
	fn join(&mut self, key: &VolumeKey, id: RequestId) {
		if let Some(parked) = self.by_key.get_mut(key)
			&& parked.job.callers.insert(id)
		{
			self.callers += 1;
		}
	}

	/// Removes fork request `id` from the parked job it joined, returning whether that job's
	/// parent last answered `TailUnproven`. Visits the parked jobs only while one has a caller,
	/// and runs only at a fork request's deadline.
	fn leave(&mut self, id: RequestId) -> Option<bool> {
		if self.callers == 0 {
			return None;
		}
		let job = self
			.by_key
			.values_mut()
			.map(|parked| &mut parked.job)
			.find(|job| job.callers.contains(&id))?;
		job.callers.remove(&id);
		self.callers -= 1;
		Some(job.tail_unproven)
	}

	fn oldest(&self) -> Option<&VolumeKey> {
		self.by_order.values().next()
	}

	fn keys(&self) -> impl Iterator<Item = &VolumeKey> {
		self.by_key.keys()
	}

	fn into_jobs(self) -> impl Iterator<Item = (VolumeKey, Job)> {
		self.by_key
			.into_iter()
			.map(|(key, parked)| (key, parked.job))
	}
}

/// Checks the parked jobs against the cap in constant time and, in debug builds, that every
/// parked job is `Phase::Parked`, ordered and counted, and that no job is both running and
/// parked.
pub(crate) fn validate_parked(state: &MaintainerState) -> anyhow::Result<()> {
	let parked = &state.forks.parked;
	anyhow::ensure!(
		parked.by_key.len() == parked.by_order.len(),
		"parked lazy fork index drifted"
	);
	anyhow::ensure!(
		parked.len() <= state.config.max_parked_lazy_fork_jobs as usize,
		"parked lazy fork capacity exceeded"
	);
	#[cfg(debug_assertions)]
	{
		let consistent = parked.by_key.iter().all(|(key, entry)| {
			matches!(entry.job.phase, Phase::Parked(_))
				&& parked.by_order.get(&entry.order) == Some(key)
				&& !state.forks.jobs.contains_key(key)
		});
		anyhow::ensure!(
			consistent,
			"a parked lazy fork job is not parked, not ordered, or also running"
		);
		let running_parked = state
			.forks
			.jobs
			.values()
			.any(|job| matches!(job.phase, Phase::Parked(_)));
		anyhow::ensure!(!running_parked, "a running fork job is parked");
		let callers: usize = parked
			.by_key
			.values()
			.map(|entry| entry.job.callers.len())
			.sum();
		anyhow::ensure!(
			callers == parked.callers,
			"parked lazy fork callers drifted"
		);
	}
	Ok(())
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
	/// The metadata a group acquire carried inline, written as the fork's one copy page.
	inline_metadata: Option<FsForkMetadataPage>,
	/// The parent's latest answer to the job's acquire was `TailUnproven`: its members hold a
	/// tail nothing proved committed. A caller is answered with that refusal, when it arrives
	/// and at a caller's deadline, instead of `Overloaded`, so a caller that can have the tail
	/// proven, such as pegboard opening a sleeping source, learns why the job waits. Any other
	/// answer from the parent clears it, and so does a non-lazy job's next acquire, since the
	/// answer to that one is the parent's latest.
	tail_unproven: bool,
}

impl Job {
	/// A bounded diagnostic label; never expose a preparation's data or credentials.
	pub(crate) fn activity_phase(&self) -> &'static str {
		match self.phase {
			Phase::Queued(_) => "QUEUED",
			Phase::Held(_) => "HELD",
			Phase::HeldInexpressible(_) => "HELD_INEXPRESSIBLE",
			Phase::Storing(_) => "STORING",
			Phase::Ready(_) => "READY",
			Phase::Peer(_) => "PEER",
			Phase::Waiting(_) => "WAITING",
			Phase::Parked(_) => "PARKED",
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
	/// The job's next step needs a store that writes a higher `depot.fs_rows` version than its
	/// durable mark: an Inspect or Prepare of a group fork before the group fork's version, or
	/// a copied page with a row only the raised mark encodes, such as a KV range whiteout. It
	/// may also wait because the grant selected an older version and never writes it. No I/O
	/// is in flight and the store queue does not hold the job. The pass that lands the mark
	/// queues the step again (`rows_landed`); a step the store refused as `Busy`, or a
	/// page refused after that pass already completed, is also queued again by the fork retry
	/// timer. A grant below the version arms neither, so the job waits for a later grant of
	/// the tablet.
	Held(StoreAction),
	/// The store refused a copied page whose row no version this grant reaches can encode, as
	/// while a storage worker of an older release is live. No I/O is in flight and nothing in
	/// this grant queues the step again: the hold arms the tablet's inexpressible hold deadline,
	/// whose firing hands the grant back, and a later grant runs the step from the job's durable
	/// preparation.
	HeldInexpressible(StoreAction),
	/// Only this operation's local completion can advance the job.
	Storing(LocalStep),
	/// Durable progress ready for the next bounded step.
	Ready(FsForkPreparation),
	/// Retains the exact image/cursor while its regional attempt runs.
	Peer(FsForkPreparation),
	/// Transport failure is uncertain; retry the same durable preparation.
	Waiting(FsForkPreparation),
	/// A lazy fork's acquire was answered `TailUnproven` and no open of the destination waits
	/// for the job: only the parent's holder writing, a recovery or a proof an opener asks
	/// for can prove that tail, so asking on a timer would only poll. No I/O is in flight, no
	/// fork retry timer is armed and the job takes no slot of the lazy fork window, so a quiet
	/// parent never keeps other children's first opens out. The job still holds the
	/// destination, and the next admitted open of it moves the job back to `Ready`
	/// (`redrive_parked`), which asks the parent again. Parked jobs are bounded by
	/// `max_parked_lazy_fork_jobs`; at that cap the oldest parked job is dropped (`drop_parked`),
	/// and an archive of the destination's group drops it too (`group_archived`). The
	/// preparation is always `FsForkAcquirePending`, and the job lives in `Forks::parked`, not
	/// `Forks::jobs`, so the passes after every event never visit it.
	Parked(FsForkPreparation),
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
	Retarget,
	SkipCopy,
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
		StoreAction::Retarget(_) => LocalStep::Retarget,
		StoreAction::SkipCopy => LocalStep::SkipCopy,
		StoreAction::RestoreTarget => LocalStep::RestoreTarget,
		StoreAction::Publish { .. } => LocalStep::Publish,
		StoreAction::Fail(_) => LocalStep::Fail,
	}
}
/// The callers of every fork job. Visits only running jobs, which the windows bound; the
/// parked jobs' callers are counted as they join and leave.
pub(crate) fn pending_requests(state: &MaintainerState) -> usize {
	let running: usize = state.forks.jobs.values().map(|j| j.callers.len()).sum();
	running + state.forks.parked.callers
}

/// The job holding `key`, running or parked.
pub(crate) fn job<'a>(state: &'a MaintainerState, key: &VolumeKey) -> Option<&'a Job> {
	state.forks.job(key)
}

pub(crate) fn holds_key(state: &MaintainerState, key: &VolumeKey) -> bool {
	job(state, key).is_some()
}

/// The ancestor volume a lazy fork job of `key` asks, when the ancestor's latest answer was
/// `TailUnproven`: an open of `key` expiring now is refused naming it, so its caller can have
/// that tail proven.
pub(crate) fn lazy_tail_unproven(state: &MaintainerState, key: &VolumeKey) -> Option<VolumeKey> {
	job(state, key)
		.filter(|job| lazy(&job.spec) && job.tail_unproven)
		.map(|job| job.spec.parent_volume_key.clone())
}

/// Whether a lazy fork job holds `key`: an open of it waits for the job instead of being
/// refused.
pub(crate) fn holds_lazy(state: &MaintainerState, key: &VolumeKey) -> bool {
	job(state, key).is_some_and(|job| lazy(&job.spec))
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
	match job(state, key) {
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
	if let Some(token) = job(state, key).and_then(|job| job.fail_log) {
		super::maintainer_log::applied(state, token, fx);
	}
	let ended = match state.forks.jobs.remove(key) {
		Some(job) => Some((job, true)),
		None => state.forks.parked.remove(key).map(|job| (job, false)),
	};
	if let Some((job, running)) = ended {
		fx.cancel_timer(MaintainerTimer::ForkRetry(key.clone()));
		let held_inexpressible = matches!(job.phase, Phase::HeldInexpressible(_));
		let lazy_window = lazy(&job.spec);
		for id in job.callers {
			respond(id, response.clone(), fx);
		}
		if held_inexpressible {
			super::filesystem::sync_inexpressible_deadline(state, fx);
		}
		// A parked job holds no slot of either window, so dropping one frees none.
		if running {
			job_freed(state, lazy_window, fx);
		}
	}
	super::lazy_fork_waiters::job_ended(state, key);
}

/// A job of the lazy fork window (`lazy_window`) or of the other window ended, so one
/// destination whose pending fork discovery skipped for want of a job of that window is
/// scanned again.
fn job_freed(state: &mut MaintainerState, lazy_window: bool, fx: &mut Effects) {
	let skipped: fn(&mut MaintainerState) -> &mut super::pending_work::Skipped = if lazy_window {
		|state| &mut state.forks.skipped_lazy
	} else {
		|state| &mut state.forks.skipped
	};
	super::pending_work::Skipped::job_ended(state, skipped, "fork_job", fx);
}

/// Whether a discovered lazy fork would get a job now.
pub(crate) fn has_free_lazy_job(state: &MaintainerState) -> bool {
	lazy_job_count(state) < state.config.max_lazy_fork_jobs as usize
}

/// Whether a discovered fork other than a lazy one would get a job now.
pub(crate) fn has_free_non_lazy_job(state: &MaintainerState) -> bool {
	non_lazy_job_count(state) < state.config.max_background_cold_reads as usize
}

/// The running jobs that are lazy forks, which take the lazy fork window. A parked job takes
/// none and is not among the running jobs. Running jobs are bounded by the windows, so this
/// visits at most their sum.
pub(crate) fn lazy_job_count(state: &MaintainerState) -> usize {
	state
		.forks
		.jobs
		.values()
		.filter(|job| lazy(&job.spec))
		.count()
}

/// The lazy fork jobs parked behind an unproven parent tail; see `Phase::Parked`.
pub(crate) fn parked_job_count(state: &MaintainerState) -> usize {
	state.forks.parked.len()
}

/// The running jobs that are not lazy forks, which share `max_background_cold_reads`.
pub(crate) fn non_lazy_job_count(state: &MaintainerState) -> usize {
	state
		.forks
		.jobs
		.values()
		.filter(|job| !lazy(&job.spec))
		.count()
}

/// Whether a job of `spec` would start now. Lazy forks have their own window,
/// `max_lazy_fork_jobs`, and every other fork job shares `max_background_cold_reads`, so a
/// wake of many fork children and an actor fork never take each other's jobs.
fn has_free_job_for(state: &MaintainerState, spec: &FsForkSpec) -> bool {
	if lazy(spec) {
		has_free_lazy_job(state)
	} else {
		has_free_non_lazy_job(state)
	}
}

/// An open of `key` was admitted to wait for its lazy fork job. A job parked behind an
/// unproven parent tail goes back to `Ready`, so the next drive asks the parent again, whose
/// tail the opener may have had proven since. Returns false when the job is parked and the
/// lazy fork window has no room: the open is refused for a retry and the job stays parked.
pub(crate) fn redrive_parked(state: &mut MaintainerState, key: &VolumeKey) -> bool {
	if state.forks.parked.get(key).is_none() {
		return true;
	}
	if lazy_job_count(state) >= state.config.max_lazy_fork_jobs as usize {
		return false;
	}
	let Some(mut job) = state.forks.parked.remove(key) else {
		return true;
	};
	if let Phase::Parked(preparation) = &job.phase {
		job.phase = Phase::Ready(preparation.clone());
	}
	state.forks.jobs.insert(key.clone(), job);
	crate::metrics::GROUP_LAZY_FORKS
		.with_label_values(&["redriven"])
		.inc();
	true
}

/// Drops parked job `key` and releases its destination; `result` is the code-defined metric
/// label naming why. The pending fork the job prepared stays in the store, so nothing is lost:
/// the destination's next open finds it in its record lookup and resumes it at its stored spec
/// (`lifecycle::row`), and so does discovery after an owner change. A parked job holds no
/// source on the parent, which answered its acquire `TailUnproven`, so nothing remote leaks.
/// A caller that joined the job is answered as at its deadline, with `TailUnproven`.
fn drop_parked(
	state: &mut MaintainerState,
	key: &VolumeKey,
	result: &'static str,
	fx: &mut Effects,
) {
	crate::metrics::GROUP_LAZY_FORKS
		.with_label_values(&[result])
		.inc();
	finish(state, key, rejected(FsForkError::TailUnproven), fx);
}

/// The group of `group`'s volumes was archived: every open of them is refused from now on, so
/// no open will re-drive their parked lazy fork jobs, which are dropped. O(parked jobs), only
/// on an archive.
pub(crate) fn group_archived(
	state: &mut MaintainerState,
	group: rivet_keys::keys::codec::GroupId,
	fx: &mut Effects,
) {
	let archived: Vec<VolumeKey> = state
		.forks
		.parked
		.keys()
		.filter(|key| {
			depot_types::keys::partition::forkable_group(&key.partition_key) == Some(group)
		})
		.cloned()
		.collect();
	for key in archived {
		drop_parked(state, &key, "parked_archived", fx);
	}
}

/// Remembers that discovery found `spec`'s pending fork while every job of its window was
/// taken.
fn skip(state: &mut MaintainerState, spec: &FsForkSpec) {
	let cap = state.config.max_loaded_volumes as usize;
	let volume = PendingVolume::of(&spec.destination_volume_key);
	if lazy(spec) {
		state.forks.skipped_lazy.skip(volume, cap);
	} else {
		state.forks.skipped.skip(volume, cap);
	}
}

/// `key`'s fork took a job, so it no longer waits in either skipped set.
fn started(state: &mut MaintainerState, key: &VolumeKey) {
	let volume = PendingVolume::of(key);
	state.forks.skipped.started(&volume);
	state.forks.skipped_lazy.started(&volume);
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
	if let Some(job) = job(state, key) {
		if job.spec != spec {
			let error = if job.spec.operation_id == spec.operation_id {
				FsForkError::OperationConflict
			} else {
				FsForkError::Busy
			};
			respond(id, rejected(error), fx);
			return;
		}
		// A request that joins a parked job waits as an open would not: only an open re-drives
		// it, so the request is answered at its deadline unless an open or the parent's holder
		// proves the tail first.
		match state.forks.jobs.get_mut(key) {
			Some(job) => {
				job.callers.insert(id);
			}
			None => state.forks.parked.join(key, id),
		}
	} else {
		// A restore of a grouped incarnation a group fork has passed would hide it from the
		// child. The loaded record and the cached group answer here; the stamp the publication
		// draws is checked again before it is logged. Only a new restore is refused: a retry
		// joins the job above, and one whose restore already published replaced the
		// incarnation, so its job's first step reads the receipt.
		if refuses_restore(state, key, &spec) {
			tracing::warn!(
				volume_key = ?key,
				"refusing a restore of a grouped volume its group forked since; its children would lose it"
			);
			respond(id, rejected(FsForkError::Invalid), fx);
			return;
		}
		// A grouped destination's rows and record carry its group's fork numbers, which only a
		// store at the group fork's row version can hold. Until the tablet's pass raises it, a
		// new job is refused before it starts, and the caller sends the same body again.
		if depot_types::keys::partition::forkable_group(&key.partition_key).is_some()
			&& !super::groups::rows_ready(state, fx)
		{
			respond(
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
		if !start_job(state, spec, BTreeSet::from([id])) {
			respond(
				id,
				ClientResponse::VolumeRejectResponse(depot_types::client::overloaded()),
				fx,
			);
			return;
		}
	}
	fx.arm_timer_at(MaintainerTimer::ForkRequest(id), deadline);
}

/// Whether a new restore `spec` of `key` replaces a grouped incarnation, still current, that a
/// group fork has passed, as the loaded record and the cached group read show.
fn refuses_restore(
	state: &MaintainerState,
	key: &depot_types::VolumeKey,
	spec: &FsForkSpec,
) -> bool {
	let Some(replaced) = spec.restore else {
		return false;
	};
	let replaced =
		depot_types::VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
			volume_key: key.clone(),
			volume_incarnation: replaced,
		});
	if super::deletion::is_replaced(state, &replaced) {
		return false;
	}
	let created_at_fork = state
		.loaded_volumes
		.get(&replaced)
		.and_then(|volume| volume.row(&replaced))
		.map(|row| row.created_at_fork);
	created_at_fork.is_some_and(|created_at_fork| {
		super::groups::known_forked_since(state, key, created_at_fork)
	})
}

/// Whether `spec` is a lazy fork: a group fork of its destination from an ancestor group.
fn lazy(spec: &FsForkSpec) -> bool {
	matches!(spec.selector, FsForkSelector::FsForkGroup(_))
}

/// Whether `spec`'s rows and record carry group fork numbers, which only a store writing the
/// group fork's `depot.fs_rows` version holds: a lazy fork, or any fork into a grouped
/// destination. Admission refuses such a fork until the mark is durable; a job that reaches
/// its store steps another way, found by discovery or resumed from the log, is held instead.
fn needs_group_rows(spec: &FsForkSpec) -> bool {
	lazy(spec)
		|| depot_types::keys::partition::forkable_group(&spec.destination_volume_key.partition_key)
			.is_some()
}

/// Queues a job of `spec` with `callers` waiting on it, or returns false when every job is
/// taken.
fn start_job(state: &mut MaintainerState, spec: FsForkSpec, callers: BTreeSet<RequestId>) -> bool {
	let key = spec.destination_volume_key.clone();
	if !has_free_job_for(state, &spec) {
		return false;
	}
	state.fork_checked.remove(&key);
	started(state, &key);
	state.store_queue.push_back(StoreWork::Fork {
		volume_key: key.clone(),
	});
	state.forks.jobs.insert(
		key,
		Job {
			spec,
			phase: Phase::Queued(StoreAction::Inspect),
			callers,
			log: None,
			replay: false,
			fail_log: None,
			restore_image: None,
			inline_metadata: None,
			tail_unproven: false,
		},
	);
	true
}

/// Starts the lazy fork `spec` for an open of its destination, which is retried while the
/// job runs: the job has no caller to answer, and the open that finds the published record
/// opens it as any existing volume. A job already running for the destination is left alone;
/// a job that cannot start now is tried by the next open.
/// Starts the lazy fork job of `spec`'s destination, or finds it running. Returns whether a job
/// for the destination runs, which an open of it then waits for.
pub(crate) fn start_lazy(
	state: &mut MaintainerState,
	spec: FsForkSpec,
	deadline: u64,
	fx: &mut Effects,
) -> bool {
	let key = spec.destination_volume_key.clone();
	if state.phase != MaintainerPhase::Serving {
		return false;
	}
	if holds_key(state, &key) {
		return true;
	}
	if deadline <= fx.now_boot_ms() {
		return false;
	}
	// The job writes rows only a store at the group fork's row version can hold. A grant that
	// selected an older version never writes them, so the open is refused for a retry under a
	// later grant. Until the tablet's pass raises the durable mark, `dispatch` holds the job's
	// first step and asks for that pass, and the open waits for the job as for any other: on a
	// fresh grant the first lazy fork comes before the pass, and its caller may not retry.
	if !super::groups::rows_selected(state) {
		return false;
	}
	if start_job(state, spec, BTreeSet::new()) {
		crate::metrics::GROUP_LAZY_FORKS
			.with_label_values(&["started"])
			.inc();
		return true;
	}
	false
}
/// Resumes the lazy fork `spec` the store holds for its destination, which no job holds, for
/// an open of that destination; see `start_lazy`. Returns false for a spec that is not a lazy
/// fork, and when no job can start now.
pub(crate) fn resume_lazy(
	state: &mut MaintainerState,
	spec: FsForkSpec,
	deadline: u64,
	fx: &mut Effects,
) -> bool {
	if !lazy(&spec) || holds_key(state, &spec.destination_volume_key) {
		return false;
	}
	let resumed = start_lazy(state, spec, deadline, fx);
	if resumed {
		crate::metrics::GROUP_LAZY_FORKS
			.with_label_values(&["resumed"])
			.inc();
	}
	resumed
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
	if state.phase != MaintainerPhase::Serving || holds_key(state, key) {
		return Ok(());
	}
	if !has_free_job_for(state, &spec) {
		skip(state, &spec);
		return Ok(());
	}
	// A grant that never writes the group fork's row version could only hold the job for its
	// whole lifetime. The fork stays durable for a grant that selects that version.
	if needs_group_rows(&spec) && state.config.write_versions.fs_rows < FORK_PIN_ROW_VERSION {
		tracing::debug!(
			?key,
			"a discovered group fork waits for a grant that writes the group fork's row version"
		);
		return Ok(());
	}
	state.fork_checked.remove(key);
	started(state, key);
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
			inline_metadata: None,
			tail_unproven: false,
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
			inline_metadata: None,
			tail_unproven: false,
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
		| Phase::Parked(preparation)
		| Phase::Logging(preparation) => preparation,
		Phase::Queued(_)
		| Phase::Held(_)
		| Phase::HeldInexpressible(_)
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

/// A lazy fork's walk found the volume absent at the fork: the job fails as a fork whose
/// parent volume does not exist, and the next open of the destination creates it as today.
fn absent(state: &mut MaintainerState, key: &VolumeKey, fx: &mut Effects) {
	mark_absent(state, key);
	crate::metrics::GROUP_LAZY_FORKS
		.with_label_values(&["absent"])
		.inc();
	fail(state, key, FsForkError::VolumeNotFound, fx);
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
	let op = rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkFailOp(
		rivet_data::generated::depot_maintainer_log_v2::ForkFailOp {
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
	// The op carries the fork the record is stamped with, so a replay stamps the same one.
	let created_at_fork = logged_created_at_fork(state, token);
	queue(
		state,
		&key,
		StoreAction::Publish {
			claimed,
			logged: true,
			incarnation_epoch,
			created_at_fork,
		},
	);
}

/// The group fork the publication op at `token` stamps its record with.
fn logged_created_at_fork(
	state: &MaintainerState,
	token: super::maintainer_log::LogToken,
) -> Option<u64> {
	match super::maintainer_log::op(state, token)? {
		rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkPublishOp(op) => {
			op.created_at_fork
		}
		_ => None,
	}
}

/// The ordering epoch the publication op at `token` gave its incarnation.
fn logged_epoch(state: &MaintainerState, token: super::maintainer_log::LogToken) -> Option<u64> {
	match super::maintainer_log::op(state, token)? {
		rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkPublishOp(op) => {
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
		Phase::Held(_)
		| Phase::HeldInexpressible(_)
		| Phase::Storing(_)
		| Phase::Ready(_)
		| Phase::Peer(_)
		| Phase::Waiting(_)
		| Phase::Parked(_)
		| Phase::Logging(_)
		| Phase::LoggingFailure(_)
		| Phase::FailureWaiting(_) => return,
	};
	let spec = job.spec.clone();
	// A group fork's Inspect or Prepare before the store writes the group fork's rows would
	// be refused by the store. The job waits for the pass that lands the raised mark, which
	// `rows_ready` asks for, whether admission, discovery or a log replay started it.
	if matches!(action, StoreAction::Inspect | StoreAction::Prepare)
		&& needs_group_rows(&spec)
		&& !super::groups::rows_ready(state, fx)
	{
		warn_if_grant_below_rows(state, &key);
		state.forks.jobs.get_mut(&key).expect("owned fork").phase = Phase::Held(action);
		return;
	}
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
		Err(error) => {
			tracing::warn!(
				%error,
				?step,
				volume_key = ?key,
				"a fork's store step failed; the maintainer fences and the fork is retried under the next grant"
			);
			super::fence(state, fx)
		}
		Ok(StoreResult::RestoreTarget(record))
			if step == LocalStep::RestoreTarget
				&& record.volume_key() == key
				&& Some(record.volume_incarnation) == spec.restore
				&& record.status == FsVolumeStatus::Live =>
		{
			log_restore(state, &key, &record, fx);
		}
		Ok(StoreResult::RestoreTarget(_)) => super::fence(state, fx),
		// The store staged nothing of the refused page, so the job runs the same step again
		// once the store can encode the row.
		Ok(StoreResult::RowsMarkPending {
			action,
			row,
			version,
		}) if step == LocalStep::Copy => {
			hold_copy(state, &key, action, ApplyHold::RowsMark, &row, version, fx);
		}
		Ok(StoreResult::RowsInexpressible {
			action,
			row,
			reachable,
		}) if step == LocalStep::Copy => {
			hold_copy(
				state,
				&key,
				action,
				ApplyHold::RowsInexpressible,
				&row,
				reachable,
				fx,
			);
		}
		// Only a copy encodes rows that can wait for a version.
		Ok(StoreResult::RowsMarkPending { .. } | StoreResult::RowsInexpressible { .. }) => {
			super::fence(state, fx)
		}
		Ok(StoreResult::Unprepared) if step == LocalStep::Inspect => {
			match available(state, &spec) {
				Ok(()) => queue(state, &key, StoreAction::Prepare),
				Err(error) => finish(state, &key, rejected(error), fx),
			}
		}
		Ok(StoreResult::Unprepared) => super::fence(state, fx),
		Ok(StoreResult::Progress(Preparation::Pending(pending))) => {
			// A retarget moved the pending fork to the next ancestor's spec, which the job
			// takes as its own; every other step keeps the spec.
			if step == LocalStep::Retarget && lazy(&pending.spec) {
				let job = state.forks.jobs.get_mut(&key).unwrap();
				job.spec = pending.spec;
				job.phase = Phase::Ready(pending.preparation);
				return;
			}
			if pending.spec != spec {
				super::fence(state, fx);
				return;
			}
			let job = state.forks.jobs.get_mut(&key).unwrap();
			// A lazy fork never pages the ancestor's metadata: the acquire carried it inline,
			// written as the one copy page, or the volume reads it through the chain.
			if lazy(&spec)
				&& step == LocalStep::Remember
				&& matches!(
					pending.preparation,
					FsForkPreparation::FsForkCopying(FsForkCopying { cursor: None, .. })
				) {
				match job.inline_metadata.take() {
					Some(page) => queue(state, &key, StoreAction::Copy { after: None, page }),
					None => queue(state, &key, StoreAction::SkipCopy),
				}
				return;
			}
			job.phase = Phase::Ready(pending.preparation);
		}
		Ok(StoreResult::Progress(Preparation::Complete(record))) => {
			if record.volume_key() != key || record.volume_incarnation != spec.operation_id {
				super::fence(state, fx);
				return;
			}
			if lazy(&spec) {
				crate::metrics::GROUP_LAZY_FORKS
					.with_label_values(&[if record.metadata_local {
						"published_inline"
					} else {
						"published_through"
					}])
					.inc();
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
			if error == FsForkError::Busy
				&& needs_group_rows(&spec)
				&& refused_busy(state, &key, step, fx)
			{
				return;
			}
			// A lazy fork that a walk already found absent stored its failure; a later open
			// of the same fork finds it at inspection and goes on to create the volume.
			if step == LocalStep::Inspect && lazy(&spec) && error == FsForkError::VolumeNotFound {
				mark_absent(state, &key);
			}
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

/// The store refused a group fork's Inspect or Prepare as `Busy`, which it does while its
/// rows are below the group fork's version and also when another fork holds the destination.
/// Returns whether the job was held for a retry; when false the refusal ends the job.
///
/// Below the durable mark the step waits for the pass that lands it. With the mark durable a
/// resumed publication is retried on the fork retry timer: its op is acknowledged, so a
/// refusal never marks it done. A live or discovered job with the mark durable is refused for
/// good, as any fork whose destination another fork holds.
fn refused_busy(
	state: &mut MaintainerState,
	key: &VolumeKey,
	step: LocalStep,
	fx: &mut Effects,
) -> bool {
	let action = match step {
		LocalStep::Inspect => StoreAction::Inspect,
		LocalStep::Prepare => StoreAction::Prepare,
		LocalStep::Remember
		| LocalStep::Copy
		| LocalStep::Retarget
		| LocalStep::SkipCopy
		| LocalStep::RestoreTarget
		| LocalStep::Publish
		| LocalStep::Fail => return false,
	};
	let rows_ready = super::groups::rows_ready(state, fx);
	let replay = state.forks.jobs[key].replay;
	if rows_ready && !replay {
		return false;
	}
	tracing::debug!(
		?key,
		?step,
		rows_ready,
		replay,
		"the store refused a group fork step as busy; the job retries it"
	);
	warn_if_grant_below_rows(state, key);
	state.forks.jobs.get_mut(key).expect("owned fork").phase = Phase::Held(action);
	if rows_ready {
		fx.arm_timer_after(
			MaintainerTimer::ForkRetry(key.clone()),
			state.config.tuning().retry_ms,
		);
	}
	true
}

/// Holds `key`'s job on a copy step the store refused to encode yet, as
/// `filesystem::hold_apply` holds an apply: `version` is the store's current row version for
/// [`ApplyHold::RowsMark`] and the highest reachable one for [`ApplyHold::RowsInexpressible`].
/// A rows mark hold asks for the prompt pass that makes the raised mark durable, whose
/// completion queues the step again (`rows_landed`); a refusal that arrives after that pass
/// completed retries on the fork retry timer instead, since the store encodes at the raised mark
/// by then. An inexpressible hold waits for a later grant.
fn hold_copy(
	state: &mut MaintainerState,
	key: &VolumeKey,
	action: StoreAction,
	hold: ApplyHold,
	row: &str,
	version: u16,
	fx: &mut Effects,
) {
	let Some(job) = state.forks.jobs.get_mut(key) else {
		return;
	};
	match hold {
		ApplyHold::RowsMark => {
			job.phase = Phase::Held(action);
			tracing::debug!(
				?key,
				row,
				version,
				"fork copy waits for the pass that raises the rows mark"
			);
			if super::maintainer_log::rows_mark_pending(state) {
				super::maintainer_log::request_mark_pass(state, fx);
			} else {
				fx.arm_timer_after(
					MaintainerTimer::ForkRetry(key.clone()),
					state.config.tuning().retry_ms,
				);
			}
		}
		ApplyHold::RowsInexpressible => {
			job.phase = Phase::HeldInexpressible(action);
			// A job enters this hold once per lifetime, so this warns once per fork.
			tracing::warn!(
				?key,
				row,
				reachable = version,
				"fork copy holds a row no depot.fs_rows version of this grant can express; a storage worker may still run an older release or the grant selected from a stale topology view, so the maintainer hands the grant back unless the hold ends first"
			);
			super::filesystem::sync_inexpressible_deadline(state, fx);
		}
	}
}

/// Whether a fork job holds a copied page no version of this grant can encode.
pub(crate) fn holds_inexpressible(state: &MaintainerState) -> bool {
	inexpressible_count(state) > 0
}

/// How many fork jobs hold a copied page no version of this grant can encode.
pub(crate) fn inexpressible_count(state: &MaintainerState) -> usize {
	state
		.forks
		.jobs
		.values()
		.filter(|job| matches!(job.phase, Phase::HeldInexpressible(_)))
		.count()
}

/// Warns as a job is held when this grant selects a `depot.fs_rows` version below the group
/// fork's: no pass of this grant can land the mark, so the job holds its slot, with no I/O,
/// until a later grant of the tablet. Called once per hold, as the job enters `Held`.
fn warn_if_grant_below_rows(state: &MaintainerState, key: &VolumeKey) {
	let selected = state.config.write_versions.fs_rows;
	if selected < FORK_PIN_ROW_VERSION {
		tracing::warn!(
			?key,
			selected,
			needed = FORK_PIN_ROW_VERSION,
			"a group fork waits for a grant that writes its depot.fs_rows version; this grant selected an older one, likely because a storage worker still reads only older rows"
		);
	}
}

/// The pass carrying the raised `depot.fs_rows` mark completed, so the store writes the group
/// fork's rows: every held job queues its step again.
pub(crate) fn rows_landed(state: &mut MaintainerState, fx: &mut Effects) {
	let held: Vec<_> = state
		.forks
		.jobs
		.iter()
		.filter_map(|(key, job)| match &job.phase {
			Phase::Held(action) => Some((key.clone(), action.clone())),
			Phase::HeldInexpressible(_)
			| Phase::Queued(_)
			| Phase::Storing(_)
			| Phase::Ready(_)
			| Phase::Peer(_)
			| Phase::Waiting(_)
			| Phase::Parked(_)
			| Phase::Logging(_)
			| Phase::LoggingFailure(_)
			| Phase::FailureWaiting(_) => None,
		})
		.collect();
	for (key, action) in held {
		fx.cancel_timer(MaintainerTimer::ForkRetry(key.clone()));
		queue(state, &key, action);
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
			Phase::Ready(preparation)
			| Phase::Peer(preparation)
			| Phase::Waiting(preparation)
			| Phase::Parked(preparation) => preparation,
			Phase::Queued(_)
			| Phase::Held(_)
			| Phase::HeldInexpressible(_)
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
		.job(&key.0.volume_key)
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
		// The parent's last answer is stale once the job asks again: a tail proven since, as
		// pegboard's open of a sleeping source proves it, must not be reported unproven to a
		// caller expiring during this round, which would make the caller prove it again. A
		// lazy fork keeps it instead: an open that joins during the round and expires before
		// the answer would otherwise be refused busy and never ask for the proof. At worst a
		// proof just made is asked for once more, one empty entry of the holder's claim.
		let job = state.forks.jobs.get_mut(&key).unwrap();
		job.phase = Phase::Peer(preparation);
		if !lazy(&job.spec) {
			job.tail_unproven = false;
		}
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
		// A replay stamps what the logged publication stamped.
		let created_at_fork = state.forks.jobs[key]
			.log
			.and_then(|token| logged_created_at_fork(state, token));
		queue(
			state,
			key,
			StoreAction::Publish {
				claimed: None,
				logged: replay,
				incarnation_epoch,
				created_at_fork,
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
	let Ok(created_at_fork) = super::groups::fork_stamp(state, key, fx) else {
		wait(state, key, preparation.clone(), fx);
		return;
	};
	let Some(incarnation_epoch) = draw_epoch(state, key, 0) else {
		wait(state, key, preparation.clone(), fx);
		return;
	};
	let op = rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkPublishOp(
		rivet_data::generated::depot_maintainer_log_v2::ForkPublishOp {
			spec: spec.clone(),
			image: ready.image.clone(),
			targets: Vec::new(),
			incarnation_epoch,
			created_at_fork,
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
		// A replay stamps what the logged publication stamped.
		let created_at_fork = state.forks.jobs[key]
			.log
			.and_then(|token| logged_created_at_fork(state, token));
		queue(
			state,
			key,
			StoreAction::Publish {
				claimed,
				logged,
				incarnation_epoch,
				created_at_fork,
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
		metadata_local: true,
	});
	if !super::maintainer_log::admits(state) {
		// The drive drains and checks the target again once the retry timer fires.
		wait(state, key, preparation, fx);
		return;
	}
	let Ok(created_at_fork) = super::groups::fork_stamp(state, key, fx) else {
		wait(state, key, preparation, fx);
		return;
	};
	// The stamp is the group's latest fork, which may have risen since admission.
	if let Some(latest_fork) = created_at_fork
		&& super::groups::restore_hides_from_children(replaced.created_at_fork, latest_fork)
	{
		tracing::warn!(
			volume_key = ?key,
			latest_fork,
			"refusing a restore of a grouped volume its group forked since; its children would lose it"
		);
		fail(state, key, FsForkError::Invalid, fx);
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
	// The replaced incarnation has one claim log, so the image's cut names claim log 0 alone.
	let after = if own_checkpoint {
		match image.link.position.root_position() {
			Some(position) => position.clone(),
			None => {
				tracing::error!(
					"restore image cut names no claim log 0 position; the fork cannot publish"
				);
				super::fence(state, fx);
				return;
			}
		}
	} else {
		replaced.cold.clone()
	};
	let targets = vec![
		rivet_data::generated::depot_maintainer_log_v2::MaintainerOpTarget {
			volume_log_key: rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: replaced.volume_incarnation,
			},
			after,
		},
	];
	let op = rivet_data::generated::depot_maintainer_log_v2::MaintainerOp::ForkPublishOp(
		rivet_data::generated::depot_maintainer_log_v2::ForkPublishOp {
			spec,
			image,
			targets,
			incarnation_epoch,
			created_at_fork,
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

/// Whether lazy job `key`, whose acquire was just answered `TailUnproven` and whose callers
/// were just answered, parks rather than asking again on the retry timer: no open waits for
/// it, its preparation is the acquire, and it resumes no logged publication.
fn parks(state: &MaintainerState, key: &VolumeKey, preparation: &FsForkPreparation) -> bool {
	let job = &state.forks.jobs[key];
	matches!(preparation, FsForkPreparation::FsForkAcquirePending)
		&& !job.replay
		&& job.log.is_none()
		&& job.fail_log.is_none()
		&& !state.awaiting_lazy_fork.contains_key(key)
}

/// Parks lazy job `key` behind its parent's unproven tail; see `Phase::Parked`. At
/// `max_parked_lazy_fork_jobs` the oldest parked job is dropped first (`drop_parked`), so a
/// stale job, whose child was destroyed or whose client gave up, never makes a newer one ask
/// on a timer. Its slot of the lazy fork window frees, so a destination whose discovery
/// skipped for want of a job is scanned again.
fn park(
	state: &mut MaintainerState,
	key: &VolumeKey,
	preparation: FsForkPreparation,
	fx: &mut Effects,
) {
	let cap = state.config.max_parked_lazy_fork_jobs as usize;
	while state.forks.parked.len() >= cap {
		let Some(oldest) = state.forks.parked.oldest().cloned() else {
			break;
		};
		// The store snapshot warns when parked jobs near the cap, once per crossing.
		tracing::debug!(
			key = ?oldest,
			cap,
			"parked lazy fork jobs at depot.groups.max_parked_lazy_fork_jobs; dropping the oldest, whose next open resumes its stored fork"
		);
		drop_parked(state, &oldest, "parked_evicted", fx);
	}
	let mut job = state.forks.jobs.remove(key).expect("owned fork");
	job.phase = Phase::Parked(preparation);
	state.forks.parked.insert(key.clone(), job);
	crate::metrics::GROUP_LAZY_FORKS
		.with_label_values(&["parked"])
		.inc();
	// Only lazy jobs park, so the lazy fork window frees.
	job_freed(state, true, fx);
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
	let Some(job) = state.forks.jobs.get_mut(&key) else {
		super::fence(state, fx);
		return;
	};
	let Phase::Peer(preparation) = job.phase.clone() else {
		super::fence(state, fx);
		return;
	};
	job.tail_unproven = matches!(
		result,
		Ok(ClientResponse::FsForkReferenceResponse(
			FsForkReferenceResponse::FsForkError(FsForkError::TailUnproven)
		))
	);
	// The parent's tail is unproven: the job's callers, and the opens waiting for a lazy job,
	// learn why now rather than at their deadlines, so an opener has the tail proven at once.
	// A lazy job nothing waits for then parks until an open re-drives it; any other job waits
	// and asks again.
	if job.tail_unproven {
		for id in std::mem::take(&mut job.callers) {
			respond(id, rejected(FsForkError::TailUnproven), fx);
		}
		if lazy(&job.spec) {
			let parent_volume_key = job.spec.parent_volume_key.clone();
			super::lifecycle::refuse_job_waiters_tail_unproven(state, &key, parent_volume_key, fx);
			if parks(state, &key, &preparation) {
				park(state, &key, preparation, fx);
				return;
			}
		}
		wait(state, &key, preparation, fx);
		return;
	}
	let job = &state.forks.jobs[&key];
	let job_spec = job.spec.clone();
	match (result, &preparation) {
		(
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkReceipt(
				receipt,
			))),
			FsForkPreparation::FsForkAcquirePending,
		) if receipt.spec == job.spec => queue(state, &key, StoreAction::Remember(receipt.image)),
		// A group acquire's receipt names the ancestor's pin; the metadata it carries is the
		// fork's one copy page, written once the image is remembered.
		(
			Ok(ClientResponse::FsForkReferenceResponse(
				FsForkReferenceResponse::FsForkGroupReceipt(group),
			)),
			FsForkPreparation::FsForkAcquirePending,
		) if group.receipt.spec == job.spec => {
			state.forks.jobs.get_mut(&key).unwrap().inline_metadata = group.metadata;
			queue(state, &key, StoreAction::Remember(group.receipt.image));
		}
		// The ancestor says nothing about the fork: the next ancestor is asked, or, past the
		// root, the volume never existed in the lineage.
		(
			Ok(ClientResponse::FsForkReferenceResponse(
				FsForkReferenceResponse::FsForkGroupContinue,
			)),
			FsForkPreparation::FsForkAcquirePending,
		) if lazy(&job_spec) => match super::groups::next_lazy_fork(state, &job_spec, fx) {
			super::groups::NextLazyFork::Next(next) => {
				queue(state, &key, StoreAction::Retarget(next))
			}
			super::groups::NextLazyFork::Root => absent(state, &key, fx),
			// The lineage is being read again: the same ancestor is asked after the retry
			// delay, by which time the next step is known.
			super::groups::NextLazyFork::Unknown => wait(state, &key, preparation, fx),
		},
		// The volume did not exist in the ancestor's group at the fork.
		(
			Ok(ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkGroupAbsent)),
			FsForkPreparation::FsForkAcquirePending,
		) if lazy(&job_spec) => absent(state, &key, fx),
		// The ancestor never held the name, which its maintainer or store answers as not found
		// before the history check: like a continue, the next ancestor is asked.
		(
			Ok(
				ClientResponse::VolumeRejectResponse(
					VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted,
				)
				| ClientResponse::FsForkReferenceResponse(FsForkReferenceResponse::FsForkError(
					FsForkError::VolumeNotFound | FsForkError::VolumeDeleted,
				)),
			),
			FsForkPreparation::FsForkAcquirePending,
		) if lazy(&job_spec) => match super::groups::next_lazy_fork(state, &job_spec, fx) {
			super::groups::NextLazyFork::Next(next) => {
				queue(state, &key, StoreAction::Retarget(next))
			}
			super::groups::NextLazyFork::Root => absent(state, &key, fx),
			// The lineage is being read again: the same ancestor is asked after the retry
			// delay, by which time the next step is known.
			super::groups::NextLazyFork::Unknown => wait(state, &key, preparation, fx),
		},
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
				| FsForkError::TailUnproven
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
	// A step the store refused as busy goes to the store again; one still below the rows
	// mark is held again at dispatch until the mark lands.
	if let Some(Job {
		phase: Phase::Held(action),
		..
	}) = state.forks.jobs.get(&key)
	{
		let action = action.clone();
		queue(state, &key, action);
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
/// A caller's deadline passed while its job runs: the caller is refused for now, as
/// `TailUnproven` while the parent's latest answer was that, and as `Overloaded` otherwise.
pub(crate) fn expire(state: &mut MaintainerState, id: RequestId, fx: &mut Effects) {
	let running = state
		.forks
		.jobs
		.values_mut()
		.find(|job| job.callers.contains(&id))
		.map(|job| {
			job.callers.remove(&id);
			job.tail_unproven
		});
	let Some(tail_unproven) = running.or_else(|| state.forks.parked.leave(id)) else {
		return;
	};
	let response = if tail_unproven {
		rejected(FsForkError::TailUnproven)
	} else {
		ClientResponse::VolumeRejectResponse(depot_types::client::overloaded())
	};
	respond(id, response, fx);
}
pub(crate) fn fence(state: &mut MaintainerState, fx: &mut Effects) {
	state.forks.skipped.clear();
	state.forks.skipped_lazy.clear();
	let parked = std::mem::take(&mut state.forks.parked);
	let running = std::mem::take(&mut state.forks.jobs);
	for (key, job) in running.into_iter().chain(parked.into_jobs()) {
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
