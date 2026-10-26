//! Owns the cache, accepted requests, idle cleanup and the join barrier for all volume runners.

use super::{
	Command, Config, Error, Request, Response, VolumeClient,
	slot::{IdleCheck, Slot},
};
use crate::{metrics, volume_requests::VolumeRequests};
use anyhow::{Result, ensure};
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use rivet_data::generated::depot_tablet_manifest_v1::TabletManifest;
use rivet_env::{ClockError, Env, TaskHandle};
use rivet_state_machine::IndexedDeadlineQueue;
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};
use tokio::sync::{Notify, Semaphore, mpsc, watch};
use universalpubsub::{PubSub, SubjectCapacities};

pub(super) struct Dependencies {
	pub env: Arc<dyn Env>,
	pub pubsub: PubSub,
	pub lifecycle: VolumeRequests,
	pub capacities: SubjectCapacities,
	pub timeout: Duration,
	pub may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	pub config: Config,
	pub _volume_capacity_metric: rivet_metrics::IntGaugeGuard,
	pub _request_capacity_metric: rivet_metrics::IntGaugeGuard,
	pub idle: Arc<IdleSignal>,
	/// Shared by every volume's replica transport, so requests to one worker coalesce.
	pub log_lanes: Arc<depot_volume::ups::LogRequestLanes>,
	/// The worker's topology, which selects the version each filesystem entry is written at.
	pub versions: Arc<dyn rivet_topology::versions::WriteVersions>,
}

/// Explicit service owner. Shutdown drains accepted work, closes volumes, and joins every runner.
/// Drop requests the same cleanup, but callers must await shutdown to prove completion.
pub struct Volumes {
	handle: VolumeClient,
	inbox: Option<mpsc::Sender<Request>>,
	stop: watch::Sender<bool>,
	joined: Option<TaskHandle>,
	env: Arc<dyn Env>,
}

impl Volumes {
	pub fn start(
		pubsub: PubSub,
		lifecycle: VolumeRequests,
		capacities: SubjectCapacities,
		timeout: Duration,
		config: Config,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Self> {
		ensure!(
			config.max_loaded_volumes > 0
				&& config.max_requests > 0
				&& config.max_requests <= Semaphore::MAX_PERMITS
				&& config.max_loaded_volumes <= i64::MAX as usize
				&& config.max_volume_key_bytes > 0,
			"volume client bounds must be positive"
		);
		ensure!(
			config.sweep_interval_ms > 0 && config.close_after_idle_ms > 0,
			"volume idle intervals must be positive"
		);
		ensure!(
			!timeout.is_zero() && capacities.incoming > 0 && capacities.outgoing > 0,
			"volume transport bounds must be positive"
		);
		ensure!(
			lifecycle.worker_id() == pubsub.worker_id(),
			"volume lifecycle and replica transports must share worker identity"
		);
		let env = pubsub.env().clone();
		let versions: Arc<dyn rivet_topology::versions::WriteVersions> = pubsub.topology().clone();
		let (inbox, receive) = mpsc::channel(config.max_requests);
		let (stop, stopping) = watch::channel(false);
		let handle = VolumeClient {
			stopping: stopping.clone(),
			lifecycle: lifecycle.clone(),
			worker_id: pubsub.worker_id().clone(),
			inbox: inbox.downgrade(),
			requests: Arc::new(Semaphore::new(config.max_requests)),
			config: config.clone(),
			may_dispatch: may_dispatch.clone(),
			versions: versions.clone(),
		};
		let log_lanes = depot_volume::ups::UpsLogClient::lanes(
			&pubsub,
			capacities,
			config.log_inflight_per_worker,
		)?;
		let dependencies = Arc::new(Dependencies {
			log_lanes,
			versions,
			env: env.clone(),
			pubsub,
			lifecycle,
			capacities,
			timeout,
			may_dispatch,
			_volume_capacity_metric: rivet_metrics::GaugeGuardExt::add_guard(
				&*metrics::VOLUME_CAPACITY,
				config.max_loaded_volumes as i64,
			),
			_request_capacity_metric: rivet_metrics::GaugeGuardExt::add_guard(
				&*metrics::REQUEST_CAPACITY,
				config.max_requests as i64,
			),
			config,
			idle: Arc::new(IdleSignal::new()),
		});
		let joined = env.spawn(Box::pin(run(dependencies, receive, stopping)));
		Ok(Self {
			handle,
			inbox: Some(inbox),
			stop,
			joined: Some(joined),
			env,
		})
	}

	pub fn handle(&self) -> VolumeClient {
		self.handle.clone()
	}

	/// Stops admission synchronously, then waits for accepted operations and volume cleanup.
	pub async fn shutdown(mut self) -> Result<()> {
		self.stop.send_replace(true);
		self.inbox.take();
		let result = if let Some(joined) = self.joined.as_mut() {
			joined.await.map_err(anyhow::Error::from)
		} else {
			Ok(())
		};
		self.joined.take();
		result
	}
}

impl Drop for Volumes {
	fn drop(&mut self) {
		self.stop.send_replace(true);
		self.inbox.take();
		if let Some(joined) = self.joined.take() {
			self.env.spawn(Box::pin(async move {
				if let Err(error) = joined.await {
					tracing::error!(?error, "volume owner failed during cleanup");
				}
			}));
		}
	}
}

/// Keys whose idle state may have changed. Lease release, a terminal runner phase, a
/// finished request and a finished release add their key; the owner drains the set into its
/// idle deadline queue. It holds at most one entry per loaded slot.
pub(super) struct IdleSignal {
	keys: scc::HashSet<depot_types::VolumeKey>,
	notify: Notify,
}

impl IdleSignal {
	fn new() -> Self {
		Self {
			keys: scc::HashSet::new(),
			notify: Notify::new(),
		}
	}

	/// Called from sync contexts such as `Drop` and runner observers, so it uses the sync insert.
	/// No async code waits on this map.
	pub(crate) fn signal(&self, key: &depot_types::VolumeKey) {
		let _ = self.keys.insert_sync(key.clone());
		self.notify.notify_one();
	}

	async fn drain(&self) -> Vec<depot_types::VolumeKey> {
		let mut keys = vec![];
		if self.keys.is_empty() {
			return keys;
		}
		self.keys
			.retain_async(|key| {
				keys.push(key.clone());
				false
			})
			.await;
		keys
	}
}

/// The request permit stays in the future, so cancellation of its caller never cancels ownership.
///
/// Idle cleanup is driven by a deadline queue of check times. A key is checked when it comes
/// due, and the check revalidates against the slot itself, so the queue never needs to be
/// exact: a held lease drops the key until its release signals again, a slot touched since is
/// rescheduled at its new idle deadline, and a contended lock retries after the sweep interval.
/// Nothing visits a slot that has no signal and no due deadline.
async fn run(
	deps: Arc<Dependencies>,
	mut inbox: mpsc::Receiver<Request>,
	mut stop: watch::Receiver<bool>,
) {
	let clock = deps.env.clock();
	let retry = Duration::from_millis(deps.config.sweep_interval_ms);
	let mut slots: BTreeMap<depot_types::VolumeKey, Arc<Slot>> = BTreeMap::new();
	let mut idle: IndexedDeadlineQueue<depot_types::VolumeKey> = IndexedDeadlineQueue::new();
	let mut pending: FuturesUnordered<BoxFuture<'static, ()>> = FuturesUnordered::new();
	// Idle releases are bounded separately so they leave lifecycle admission to active requests.
	let release_limit = deps
		.config
		.max_idle_releases
		.min(deps.config.max_requests)
		.max(1);
	let mut releases: FuturesUnordered<BoxFuture<'static, depot_types::VolumeKey>> =
		FuturesUnordered::new();
	// Held volumes learn of a replica replacement from the manifest refresh the process runs
	// anyway, so a writer that writes nothing matches new members itself instead of waiting
	// for the maintainer's recovery, which would claim the volume and fence it.
	let mut manifests = deps.lifecycle.manifests();
	let mut handed_off_version = manifest_version(&manifests);
	// A watcher that stopped ends the hand-off; waiting on it again would spin.
	let mut manifests_live = true;

	loop {
		let now = match clock.observe() {
			Ok(now) => now.boot_ms,
			Err(_) => {
				inbox.close();
				break;
			}
		};
		if *stop.borrow() {
			inbox.close();
			break;
		}

		// Every signalled key is checked now; the check itself decides its next deadline.
		for key in deps.idle.drain().await {
			if slots.contains_key(&key) {
				idle.schedule_earliest(key, now);
			}
		}

		// A full release bound stops idle checks until a release finishes.
		let idle_at = if releases.len() < release_limit {
			idle.next_at().unwrap_or(u64::MAX)
		} else {
			u64::MAX
		};
		let sweep_at = idle_at;

		let step = rivet_env::select_biased!("finish owned work and shutdown before admitting more requests",
			_ = stop.changed() => None,
			step = next_step(
				&mut pending,
				&mut releases,
				clock.sleep_until(sweep_at),
				deps.idle.notify.notified(),
				rivet_env::tokio::when(manifests_live, manifests.changed()),
				&mut inbox,
			) => Some(step),
		);
		match step {
			None => {
				inbox.close();
				break;
			}
			Some(Step::Completed) | Some(Step::Signal) => {}
			Some(Step::ManifestChanged(Err(_))) => manifests_live = false,
			Some(Step::ManifestChanged(Ok(()))) => {
				// A refresh that found the same manifest changes nothing for held volumes.
				let version = manifest_version(&manifests);
				if version != handed_off_version {
					handed_off_version = version;
					hand_off_replica_sets(&deps, slots.values());
				}
			}
			Some(Step::Released(key)) => {
				// A successful release leaves the slot empty; a failed one retries later.
				if let Some(slot) = slots.get(&key) {
					if slot.is_empty() && Arc::strong_count(slot) == 1 {
						slots.remove(&key);
						idle.cancel(&key);
					} else if let Ok(at) = rivet_env::deadline_after(clock.as_ref(), retry) {
						idle.schedule_earliest(key, at);
					}
				}
			}
			Some(Step::Request(Some(request))) => {
				admit(&deps, &mut slots, &mut pending, request);
			}
			Some(Step::Request(None)) => break,
			Some(Step::Sweep) => {
				let Ok(now) = clock.observe().map(|now| now.boot_ms) else {
					continue;
				};
				let visits = check_idle(
					now,
					deps.config.sweep_interval_ms,
					&mut slots,
					&mut idle,
					&mut releases,
					release_limit,
					|slot, now| slot.idle_check(&deps, now),
					|slot| Arc::strong_count(slot) == 1,
				);
				metrics::record_idle_visits("volume_owner_sweep", visits);
			}
		}
	}

	// Requests accepted before shutdown still own their work and receive their original results.
	while let Some(request) = inbox.recv().await {
		admit(&deps, &mut slots, &mut pending, request);
	}
	// Accepted requests can wait on an idle release's slot lock, so poll both together.
	pending.extend(
		releases
			.into_iter()
			.map(|release| release.map(|_| ()).boxed()),
	);
	while pending.next().await.is_some() {}
	let mut closing = futures_util::stream::iter(slots.into_values())
		.map(|slot| {
			let deps = deps.clone();
			async move { slot.shutdown(&deps).await }
		})
		.buffer_unordered(release_limit);
	while closing.next().await.is_some() {}
}

/// Checks due idle keys until none are due or the release bound is full, and returns how
/// many it visited. Each outcome either reschedules the key, drops it with a re-entry path
/// (a lease release or request completion signals it again), or removes its empty slot.
///
/// `check` revalidates one slot and `removable` reports whether nothing else references it.
fn check_idle<S>(
	now: u64,
	retry_ms: u64,
	slots: &mut BTreeMap<depot_types::VolumeKey, S>,
	idle: &mut IndexedDeadlineQueue<depot_types::VolumeKey>,
	releases: &mut FuturesUnordered<BoxFuture<'static, depot_types::VolumeKey>>,
	release_limit: usize,
	check: impl Fn(&S, u64) -> IdleCheck,
	removable: impl Fn(&S) -> bool,
) -> usize {
	let mut visits = 0;
	while releases.len() < release_limit {
		let Some((_, key)) = idle.pop_due(now, 1).pop() else {
			break;
		};
		visits += 1;
		let Some(slot) = slots.get(&key) else {
			continue;
		};
		match check(slot, now) {
			IdleCheck::Held => {}
			IdleCheck::NotYetDue { at } => idle.schedule(key, at),
			IdleCheck::Busy => idle.schedule(key, now.saturating_add(retry_ms)),
			IdleCheck::Empty => {
				if removable(slot) {
					slots.remove(&key);
				}
			}
			IdleCheck::Release(release) => {
				releases.push(
					async move {
						release.await;
						key
					}
					.boxed(),
				);
			}
		}
	}
	visits
}

/// One owner loop wake-up, chosen by [`next_step`].
#[derive(Debug)]
enum Step<T> {
	/// A pending operation finished.
	Completed,

	/// An idle release for this key finished, successfully or not.
	Released(depot_types::VolumeKey),

	/// The inbox yielded a request, or `None` once every sender is gone.
	Request(Option<T>),

	/// The earliest idle check deadline passed.
	Sweep,

	/// The process's manifest watcher refreshed its tablet manifest, or stopped.
	ManifestChanged(std::result::Result<(), watch::error::RecvError>),

	/// A key was signalled idle; the loop drains it before waiting again.
	Signal,
}

/// Selects the next owner step in strict priority order: completions, sweep, signals, then
/// admission.
///
/// Every arm before admission is bounded. Completions only drain work that admission or
/// sweeps pushed, and the sweep fires only at a due deadline, so admission is never starved;
/// admission itself is capped by the request semaphore.
async fn next_step<T>(
	pending: &mut FuturesUnordered<BoxFuture<'static, ()>>,
	releases: &mut FuturesUnordered<BoxFuture<'static, depot_types::VolumeKey>>,
	sweep: impl Future<Output = Result<(), ClockError>>,
	signal: impl Future<Output = ()>,
	manifest: impl Future<Output = std::result::Result<(), watch::error::RecvError>>,
	inbox: &mut mpsc::Receiver<T>,
) -> Step<T> {
	let pending_enabled = !pending.is_empty();
	let releases_enabled = !releases.is_empty();
	rivet_env::select_biased!("complete bounded work before admitting more of it",
		_ = rivet_env::tokio::when(pending_enabled, pending.next()) => Step::Completed,
		key = rivet_env::tokio::when(releases_enabled, releases.next()) => match key {
			Some(key) => Step::Released(key),
			None => Step::Completed,
		},
		step = async {
			rivet_env::select_biased!("idle cleanup must not starve under request traffic",
				_ = sweep => Step::Sweep,
				step = async {
					rivet_env::select_biased!("a signalled key is checked before a manifest change",
						_ = signal => Step::Signal,
						changed = manifest => Step::ManifestChanged(changed),
					)
				} => step,
				request = inbox.recv() => Step::Request(request),
			)
		} => step,
	)
}

/// The version of the cached manifest, or `None` before the watcher has one.
fn manifest_version(manifests: &watch::Receiver<Option<TabletManifest>>) -> Option<u64> {
	manifests
		.borrow()
		.as_ref()
		.map(|manifest| manifest.tablet_manifest_version)
}

/// Hands every held volume's client the replica set its tablet's cached manifest names. It is
/// local and sends nothing; each client takes only a newer replica set version.
fn hand_off_replica_sets<'a>(deps: &Arc<Dependencies>, slots: impl Iterator<Item = &'a Arc<Slot>>) {
	for slot in slots {
		let Some((key, Some(handle))) = slot.held() else {
			continue;
		};
		let Ok(tablet) = deps.lifecycle.tablet(&key.0.volume_key) else {
			continue;
		};
		handle.observe_replica_set(tablet.replica_set.clone());
	}
}

/// The loaded volume count at which the owner warns once that it nears its cap: 90 percent.
pub(crate) fn near_capacity(capacity: usize) -> usize {
	(capacity / 10 * 9).max(1)
}

/// Cache admission is synchronous and bounded; each key has one slot before any open can start.
fn admit(
	deps: &Arc<Dependencies>,
	slots: &mut BTreeMap<depot_types::VolumeKey, Arc<Slot>>,
	pending: &mut FuturesUnordered<BoxFuture<'static, ()>>,
	request: Request,
) {
	let key = request.command.key().clone();
	if matches!(request.command, Command::ForkMinimum { .. }) && !slots.contains_key(&key) {
		let _ = request.reply.send(Ok(Response::ForkMinimum(None)));
		return;
	}
	if !slots.contains_key(&key) && slots.len() >= deps.config.max_loaded_volumes {
		metrics::REJECTED.with_label_values(&["volumes"]).inc();
		let _ = request.reply.send(Err(Error::Overloaded));
		return;
	}
	let new_slot = !slots.contains_key(&key);
	let slot = slots
		.entry(key)
		.or_insert_with(|| Arc::new(Slot::new()))
		.clone();
	if new_slot && slots.len() == near_capacity(deps.config.max_loaded_volumes) {
		tracing::warn!(
			loaded = slots.len(),
			capacity = deps.config.max_loaded_volumes,
			"loaded volumes near capacity; raise depot.volume_client.max_loaded_volumes if opens start failing"
		);
	}
	if new_slot && slots.len() == deps.config.max_loaded_volumes {
		tracing::warn!(
			capacity = deps.config.max_loaded_volumes,
			"volume cache capacity reached; waiting for idle cleanup"
		);
	}
	let deps = deps.clone();
	let key = request.command.key().clone();
	pending.push(
		async move {
			let result = match request.command {
				Command::ForkMinimum { .. } => {
					slot.fork_minimum(&deps, request.may_dispatch.clone()).await
				}
				Command::Checkpoint { key, name } => {
					slot.checkpoint(&deps, key, name, request.may_dispatch.clone())
						.await
				}
				Command::CheckpointUpdate { key, update } => {
					slot.checkpoint_update(&deps, key, update, request.may_dispatch.clone())
						.await
				}
				Command::Open {
					key,
					create_if_absent,
					initial,
					prefetch,
				} => {
					slot.open_filesystem(
						&deps,
						key,
						create_if_absent,
						initial,
						prefetch,
						request.may_dispatch.clone(),
					)
					.await
				}
				Command::Create { key, retention } => {
					slot.create(&deps, key, retention, request.may_dispatch.clone())
						.await
				}
				Command::Delete { key } => {
					slot.delete(&deps, key, request.may_dispatch.clone()).await
				}
			};
			let _ = request.reply.send(result);
			drop(request._permit);
			// The request may have left the slot idle, empty or without a held lease.
			drop(slot);
			deps.idle.signal(&key);
		}
		.boxed(),
	);
}


