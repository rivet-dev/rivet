//! Depot schedules a bounded full merge when an idle tablet has a reclaimable delete.

use bytes::Bytes;
use depot_fs_core::keys::slatedb::{
	IncarnationPrefix, Scope, decode_key, row_stamp, versioned_row_prefix_len,
};
use parking_lot::Mutex;
use rivet_env::Clock;
use rivet_keys::keys::PrefixAddress;
use slatedb::{
	compactor::{
		CompactionRequest, CompactionScheduler, CompactionSchedulerSupplier, CompactionSpec,
		CompactionStatus, CompactorStateView, SizeTieredCompactionSchedulerSupplier,
	},
	config::{
		CompactorOptions, DurabilityLevel, ScanOptions, SizeTieredCompactionSchedulerOptions,
	},
};
use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::{
	filter::{Catalog, Retention},
	store::FilesystemStore,
};

const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
const MIN_INTERVAL: Duration = Duration::from_mins(15);
const PROBE_READ_AHEAD_BYTES: usize = 128 * 1024;
const CLEANUP_CONCURRENCY: usize = 2;

/// Independent probe and merge limits shared by every tablet on a storage worker.
#[derive(Clone, Debug)]
pub struct CleanupBudget {
	probes: Arc<Semaphore>,
	merges: Arc<Semaphore>,
}

impl Default for CleanupBudget {
	fn default() -> Self {
		Self {
			probes: Arc::new(Semaphore::new(CLEANUP_CONCURRENCY)),
			merges: Arc::new(Semaphore::new(CLEANUP_CONCURRENCY)),
		}
	}
}

pub(super) struct Supplier {
	pub(super) catalog: Catalog,
	pub(super) clock: Arc<dyn Clock>,
	pub(super) reclamation: Arc<Reclamation>,
}

pub(crate) fn supports_reclamation(options: &CompactorOptions) -> bool {
	// Keep a slot for L0. Trivial moves can complete without filtering any rows.
	if options.enable_trivial_move
		|| options.max_concurrent_compactions < 2
		|| options
			.worker
			.as_ref()
			.is_some_and(|worker| worker.max_concurrent_compactions < 2)
	{
		return false;
	}

	let tiered = SizeTieredCompactionSchedulerOptions::from(&options.scheduler_options);
	let min = tiered.min_compaction_sources;
	// The idle tablet has fewer than min compatible old runs, or the native
	// scheduler would already have work. If L0 backs up at max compatible runs
	// while cleanup holds the old inputs, at least min must be fresh so native
	// compaction can merge them independently.
	tiered
		.max_compaction_sources
		.saturating_sub(min.saturating_sub(1))
		>= min
}

impl CompactionSchedulerSupplier for Supplier {
	fn compaction_scheduler(
		&self,
		options: &CompactorOptions,
	) -> Box<dyn CompactionScheduler + Send + Sync> {
		let tiered = SizeTieredCompactionSchedulerSupplier::new().compaction_scheduler(options);
		if !supports_reclamation(options) {
			return tiered;
		}
		self.reclamation.enabled.store(true, Ordering::Relaxed);
		Box::new(Scheduler {
			tiered,
			catalog: self.catalog.clone(),
			clock: self.clock.clone(),
			reclamation: self.reclamation.clone(),
		})
	}
}

struct Scheduler {
	tiered: Box<dyn CompactionScheduler + Send + Sync>,
	catalog: Catalog,
	clock: Arc<dyn Clock>,
	reclamation: Arc<Reclamation>,
}

impl CompactionScheduler for Scheduler {
	fn propose(&self, view: &CompactorStateView) -> Vec<CompactionSpec> {
		let ordinary = self.tiered.propose(view);
		let manifest = view.manifest();
		let mut state = self.reclamation.state.lock();
		state.forget_finished_rewrite(view);
		let Some(now) = self.clock.boot_ms().ok().filter(|now| {
			ordinary.is_empty()
				&& *now >= state.next_compaction_boot_ms
				&& can_reclaim(manifest)
				&& !view
					.compactions()
					.is_some_and(|jobs| jobs.recent_compactions().any(|job| job.active()))
		}) else {
			state.pause();
			return ordinary;
		};

		let durable = manifest.last_l0_seq();
		match std::mem::take(&mut state.probe) {
			Probe::Allowed(admission) if admission.durable == durable => {
				state.probe = Probe::Allowed(admission);
				return ordinary;
			}
			Probe::Found {
				scope,
				claim_log,
				seq,
				durable: scanned_through,
			} => {
				// Recheck this volume's retention. The filter still owns the deletion decision.
				drop(state);
				let ready_at = self
					.catalog
					.lock()
					.get(&scope)
					.and_then(|retention| ready_at(retention, claim_log, seq, durable));
				state = self.reclamation.state.lock();
				state.queue_retry(scope, 0);
				// Only add a sorted-run rewrite. Native compaction drains L0.
				if scanned_through == durable
					&& ready_at.is_some_and(|at| at <= now)
					&& let Ok(specs) = self.tiered.generate(
						view,
						&CompactionRequest::FullSegment {
							segment: Default::default(),
						},
					) && specs.len() == 1
				{
					state.rewrite = Some(Rewrite::Proposed(specs[0].clone()));
					state.next_compaction_boot_ms =
						now.saturating_add(MIN_INTERVAL.as_millis() as u64);
					return specs;
				}
			}
			_ => {}
		}
		// A denied reservation leaves the request due on the next scheduler poll.
		if state.pending.values().any(|at| *at <= now)
			&& let Ok(permit) = self.reclamation.budget.probes.clone().try_acquire_owned()
		{
			state.probe = Probe::Allowed(Admission {
				durable,
				permit: Arc::new(permit),
			});
			self.reclamation.ready.notify_one();
		}
		ordinary
	}

	fn validate(
		&self,
		view: &CompactorStateView,
		spec: &CompactionSpec,
	) -> Result<(), slatedb::Error> {
		self.tiered.validate(view, spec)?;
		let mut state = self.reclamation.state.lock();
		state.forget_finished_rewrite(view);
		if matches!(&state.rewrite, Some(Rewrite::Proposed(proposed)) if proposed == spec)
			&& let Some(job) = view.compactions().and_then(|jobs| {
				jobs.recent_compactions()
					.find(|job| job.status() == CompactionStatus::Submitted && job.spec() == spec)
			}) && let Some(destination) = spec.destination()
		{
			state.rewrite = Some(Rewrite::Active {
				id: job.id().into(),
				destination,
			});
		}
		Ok(())
	}
}

fn can_reclaim(manifest: &slatedb::VersionedManifest) -> bool {
	manifest.l0().is_empty()
		&& manifest.segments().is_empty()
		&& !manifest.compacted().is_empty()
		&& manifest
			.compacted()
			.iter()
			.map(|run| run.estimate_size())
			.try_fold(MAX_INPUT_BYTES, u64::checked_sub)
			.is_some()
}

#[derive(Clone)]
struct Admission {
	durable: u64,
	// An interrupted scan keeps its permit until the reader observes cancellation.
	permit: Arc<OwnedSemaphorePermit>,
}

#[derive(Default)]
enum Probe {
	#[default]
	Paused,
	Allowed(Admission),
	Found {
		scope: Scope,
		claim_log: u16,
		seq: u64,
		durable: u64,
	},
}

enum Rewrite {
	Proposed(CompactionSpec),
	Active { id: u128, destination: u32 },
}

#[derive(Default)]
struct State {
	pending: BTreeMap<Scope, u64>,
	waiting_for_cold: BTreeMap<Scope, BTreeMap<u16, u64>>,
	probe: Probe,
	rewrite: Option<Rewrite>,
	next_compaction_boot_ms: u64,
}

impl State {
	fn forget_finished_rewrite(&mut self, view: &CompactorStateView) {
		let Some(rewrite) = &self.rewrite else { return };
		let active = view.compactions().is_some_and(|jobs| {
			jobs.recent_compactions()
				.filter(|job| job.active())
				.any(|job| match rewrite {
					Rewrite::Proposed(spec) => job.spec() == spec,
					Rewrite::Active { id, .. } => u128::from(job.id()) == *id,
				})
		});
		if !active {
			self.rewrite = None;
		}
	}

	fn owns(&self, admission: &Admission) -> bool {
		matches!(&self.probe, Probe::Allowed(current) if Arc::ptr_eq(&current.permit, &admission.permit))
	}

	fn pause(&mut self) {
		if let Probe::Found { scope, .. } = std::mem::take(&mut self.probe) {
			self.queue_retry(scope, 0);
		}
	}

	fn queue_retry(&mut self, scope: Scope, at_boot_ms: u64) {
		self.pending.entry(scope).or_insert(at_boot_ms);
	}
}

/// Coalesces volume requests between the retention catalog, filter and idle compactor.
#[derive(Default)]
pub struct Reclamation {
	enabled: AtomicBool,
	budget: CleanupBudget,
	state: Mutex<State>,
	ready: tokio::sync::Notify,
}

impl Reclamation {
	pub(super) fn with_budget(budget: CleanupBudget) -> Self {
		Self {
			budget,
			..Self::default()
		}
	}

	// Only a rewrite proposed and validated by this tablet owner takes a permit.
	// Native and inherited jobs use SlateDB's existing execution policy.
	pub(super) fn merge_permits(&self, destination: u32) -> Option<Arc<Semaphore>> {
		matches!(
			self.state.lock().rewrite,
			Some(Rewrite::Active { destination: active, .. }) if active == destination
		)
		.then(|| self.budget.merges.clone())
	}

	pub(super) fn wait_for_cold(&self, catalog: &Catalog, scope: &Scope, claim_log: u16, seq: u64) {
		if !self.enabled.load(Ordering::Relaxed) {
			return;
		}
		// Use the same catalog -> reclamation lock order as publication.
		let catalog = catalog.lock();
		let Some(retention) = catalog.get(scope) else {
			return;
		};
		let mut state = self.state.lock();
		if retention.cold_of(claim_log) >= seq {
			state.pending.insert(scope.clone(), 0);
		} else {
			let waiting = state
				.waiting_for_cold
				.entry(scope.clone())
				.or_default()
				.entry(claim_log)
				.or_insert(seq);
			*waiting = (*waiting).min(seq);
		}
	}

	pub(super) fn wake_for_cold(&self, scope: &Scope, claim_log: u16, cold: u64) {
		let mut state = self.state.lock();
		let Some(waiting) = state.waiting_for_cold.get_mut(scope) else {
			return;
		};
		if waiting.get(&claim_log).is_some_and(|seq| *seq <= cold) {
			waiting.remove(&claim_log);
			if waiting.is_empty() {
				state.waiting_for_cold.remove(scope);
			}
			state.pending.insert(scope.clone(), 0);
		}
	}

	pub(super) fn forget(&self, scope: &Scope) {
		let mut state = self.state.lock();
		state.waiting_for_cold.remove(scope);
		state.pending.remove(scope);
	}

	pub(super) fn queue(&self, scope: &Scope) {
		if self.enabled.load(Ordering::Relaxed) {
			self.state.lock().pending.insert(scope.clone(), 0);
		}
	}

	pub fn start(
		self: &Arc<Self>,
		env: &dyn rivet_env::Env,
		store: Arc<FilesystemStore>,
	) -> ReclamationProbe {
		let reclamation = self.clone();
		ReclamationProbe(Some(env.spawn(Box::pin(async move {
			loop {
				reclamation.ready.notified().await;
				reclamation.inspect_pending(&store).await;
			}
		}))))
	}

	// Called after the coordinator and embedded worker have both joined.
	pub(super) fn release_after_close(&self) {
		let mut state = self.state.lock();
		state.pause();
		state.rewrite = None;
	}

	async fn inspect_pending(&self, store: &FilesystemStore) -> Option<bool> {
		let (admission, now, scopes) = {
			let mut state = self.state.lock();
			let Probe::Allowed(admission) = &state.probe else {
				return None;
			};
			let admission = admission.clone();
			let Ok(now) = store.clock.boot_ms() else {
				state.pause();
				return None;
			};
			let mut scopes = Vec::new();
			state.pending.retain(|scope, at| {
				if *at > now {
					return true;
				}
				scopes.push(scope.clone());
				false
			});
			(admission, now, scopes)
		};

		let mut found = false;
		for scope in scopes {
			let result = if self.still_allowed(store, &admission) {
				match self.inspect_scope(store, &scope, &admission, now).await {
					Ok(result) => result,
					Err(error) => {
						tracing::warn!(%error, "failed to inspect stranded Depot delete markers");
						Inspection::Retry(now.saturating_add(MIN_INTERVAL.as_millis() as u64))
					}
				}
			} else {
				Inspection::Retry(0)
			};
			let mut state = self.state.lock();
			if !state.owns(&admission) {
				state.queue_retry(scope, 0);
				continue;
			}
			match result {
				Inspection::Found { claim_log, seq } => {
					state.probe = Probe::Found {
						scope,
						claim_log,
						seq,
						durable: admission.durable,
					};
					found = true;
				}
				Inspection::Retry(at) => state.queue_retry(scope, at),
				Inspection::Clear => {}
			}
		}
		let mut state = self.state.lock();
		if state.owns(&admission) {
			state.pause();
		}
		Some(found)
	}

	fn still_allowed(&self, store: &FilesystemStore, admission: &Admission) -> bool {
		store.guard.may_send() && self.state.lock().owns(admission)
	}

	async fn inspect_scope(
		&self,
		store: &FilesystemStore,
		scope: &Scope,
		admission: &Admission,
		now: u64,
	) -> anyhow::Result<Inspection> {
		let durable = admission.durable;
		let manifest = store.db.manifest();
		if manifest.last_l0_seq() != durable || !can_reclaim(&manifest) {
			return Ok(Inspection::Retry(0));
		}
		{
			let catalog = store.catalog.lock();
			let Some(retention) = catalog.get(scope) else {
				return Ok(Inspection::Clear);
			};
			if !retention.knows_base() || retention.has_base || !retention.has_cold() {
				return Ok(Inspection::Clear);
			}
			if retention
				.cold_barrier
				.is_none_or(|barrier| barrier > durable)
			{
				return Ok(Inspection::Retry(0));
			}
		}
		let mut rows = store
			.db
			.scan_prefix_with_options(
				IncarnationPrefix::new(scope.hash, scope.name.clone(), scope.incarnation).pack(),
				..,
				&ScanOptions::default()
					.with_durability_filter(DurabilityLevel::Remote)
					.with_read_ahead_bytes(PROBE_READ_AHEAD_BYTES),
			)
			.await?;
		let mut result = Inspection::Clear;
		let mut future_deletes = BTreeMap::<u16, u64>::new();
		let mut previous = Bytes::new();
		let mut checked = 0;
		while let Some(row) = rows.next().await? {
			if checked % 128 == 0 && !self.still_allowed(store, admission) {
				return Ok(Inspection::Retry(0));
			}
			checked += 1;
			let Some(prefix_len) = versioned_row_prefix_len(&row.key) else {
				continue;
			};
			let prefix = row.key.slice(..prefix_len);
			if prefix == previous {
				continue;
			}
			// Ignore a delete superseded by a later write to the same logical key.
			previous = prefix;
			let Ok((_, key, seq)) = decode_key(&row.key) else {
				continue;
			};
			let Ok((claim_log, true)) = row_stamp(&key, row.value) else {
				continue;
			};
			let catalog = store.catalog.lock();
			let Some(retention) = catalog.get(scope) else {
				continue;
			};
			if seq > retention.cold_of(claim_log) && retention.permits_delete(claim_log, seq) {
				let pending = future_deletes.entry(claim_log).or_insert(seq);
				*pending = (*pending).min(seq);
			}
			let Some(at) = ready_at(retention, claim_log, seq, durable) else {
				continue;
			};
			if at <= now {
				result = Inspection::Found { claim_log, seq };
				break;
			}
			// The oldest deadline at which at least one marker can be reclaimed.
			result = Inspection::Retry(match result {
				Inspection::Retry(previous) => at.min(previous),
				_ => at,
			});
		}
		if store.db.manifest().id() != manifest.id() || !self.still_allowed(store, admission) {
			return Ok(Inspection::Retry(0));
		}
		for (claim_log, seq) in future_deletes {
			self.wait_for_cold(&store.catalog, scope, claim_log, seq);
		}
		Ok(result)
	}
}

enum Inspection {
	Clear,
	Retry(u64),
	Found { claim_log: u16, seq: u64 },
}

fn ready_at(retention: &Retention, claim_log: u16, seq: u64, durable: u64) -> Option<u64> {
	if seq > retention.cold_of(claim_log)
		|| !retention.permits_delete(claim_log, seq)
		|| retention
			.cold_barrier
			.is_none_or(|barrier| barrier > durable)
	{
		return None;
	}
	Some(
		retention
			.sessions
			.values()
			.filter(|(pin, _)| !pin.contains(claim_log, seq))
			.map(|(_, expires)| *expires)
			.max()
			.unwrap_or(0),
	)
}

/// The cold store stops this reader before it closes the database.
pub struct ReclamationProbe(Option<rivet_env::TaskHandle>);

impl ReclamationProbe {
	pub async fn stop(mut self) {
		if let Some(task) = self.0.take() {
			task.abort();
			if let Err(error) = task.join().await
				&& !error.is_cancelled()
			{
				tracing::error!(%error, "Depot reclamation probe stopped unexpectedly");
			}
		}
	}
}

impl Drop for ReclamationProbe {
	fn drop(&mut self) {
		if let Some(task) = &self.0 {
			task.abort();
		}
	}
}
