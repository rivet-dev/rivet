use crate::config::TopologyConfig;
use crate::heartbeat::HeartbeatState;
use crate::topology_machine::TopologyStateMachine;
use crate::topology_machine::event::Event;
use crate::topology_machine::executor::{ResponseSender, Responses, TopologyExecutor};
use crate::topology_machine::snapshot::TopologySnapshot;
use crate::topology_machine::state::State;
use crate::topology_machine::types::{FenceReason, LeaseStatus, RequestError, WorkerRegistration};
use crate::worker::{Worker, WorkerId};
use anyhow::{Context, Result, ensure};
use object_store::ObjectStore;
use rivet_env::{Env, TaskScope};
use rivet_metrics::GaugeGuardExt;
use rivet_state_machine_ephemeral::{Runner, RunnerHandle};
use std::{
	sync::{
		Arc, OnceLock,
		atomic::{AtomicU64, AtomicUsize, Ordering},
	},
	time::Duration,
};
use tokio::sync::{Mutex, Semaphore, mpsc, oneshot, watch};

enum Runtime {
	New,
	Running(RunnerHandle<TopologyStateMachine>),
	Stopped,
}

impl Runtime {
	fn running(&self) -> Option<&RunnerHandle<TopologyStateMachine>> {
		match self {
			Self::Running(r) => Some(r),
			_ => None,
		}
	}
}

/// Owns the topology runner. Reads use published snapshots; only misses and
/// explicit mutations enter the event loop. Dropping the last handle stops I/O.
pub struct Topology {
	store: Arc<dyn ObjectStore>,
	env: Arc<dyn Env>,
	runner: Mutex<Runtime>,
	snapshot: OnceLock<watch::Receiver<TopologySnapshot>>,
	tasks: Arc<TaskScope>,
	lookups: Arc<TaskScope>,
	responses: Arc<Responses>,
	admission: Arc<Semaphore>,
	/// Bounds endpoint invalidations separately so a burst of failures cannot exhaust
	/// lookup admission.
	invalidations: Arc<Semaphore>,
	next_request: AtomicU64,
	/// Lets timeout reports reach the inbox without keeping the runner alive at shutdown.
	timeout_inbox: OnceLock<mpsc::WeakSender<Event>>,
	/// Timeout reports in the inbox; the executor decrements it as each one is handled.
	timeout_reports: Arc<AtomicUsize>,
	started_at_unix_ms: i64,
	worker_leases: crate::worker_lease::WorkerLeases,
	/// Coalesced process watches; see `watch`.
	watches: Arc<crate::watch::WatchRegistry>,
	config: TopologyConfig,
}

impl Topology {
	/// Constructs idle capabilities. Call `start` explicitly with permanent observer mode
	/// or the worker registration; reads and refreshes never start background tasks.
	pub fn new(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		config: TopologyConfig,
	) -> Result<Arc<Self>> {
		config.validate()?;
		let started_at_unix_ms = env.clock().observe()?.unix_ms;
		let lookup_request_capacity = config.lookup_request_capacity;
		let tasks = Arc::new(env.task_scope());
		let lookups = Arc::new(env.task_scope());

		let watches = Arc::new(crate::watch::WatchRegistry::new(
			config.watch.capacity,
			Arc::downgrade(&tasks),
		));

		Ok(Arc::new(Self {
			watches,
			started_at_unix_ms,
			env,
			store,
			config: config.clone(),
			runner: Mutex::new(Runtime::New),
			snapshot: OnceLock::new(),
			tasks,
			lookups,
			responses: Arc::new(Mutex::new(Default::default())),
			admission: Arc::new(Semaphore::new(lookup_request_capacity)),
			invalidations: Arc::new(Semaphore::new(config.invalidation_request_capacity)),
			next_request: AtomicU64::new(0),
			timeout_inbox: OnceLock::new(),
			timeout_reports: Arc::new(AtomicUsize::new(0)),
			worker_leases: crate::worker_lease::WorkerLeases::new(config.worker_lease.clone()),
		}))
	}

	/// When this worker's topology instance was constructed; diagnostic only.
	pub fn started_at_unix_ms(&self) -> i64 {
		self.started_at_unix_ms
	}

	/// Starts once with a fixed registration, or `None` for a permanent observer.
	/// Bind the peer listener before advertising a worker. A second start is always rejected.
	pub async fn start(&self, worker_registration: Option<WorkerRegistration>) -> Result<()> {
		let registered = worker_registration.is_some();
		{
			let mut runtime = self.runner.lock().await;
			ensure!(
				matches!(*runtime, Runtime::New),
				"topology already started or stopped"
			);
			let config = &self.config;
			let state = State::new(config.clone(), worker_registration)?;
			let runner = Runner::spawn_with_executor(self.env.clone(), state, |snapshot| {
				Arc::new(TopologyExecutor {
					store: self.store.clone(),
					tasks: Arc::downgrade(&self.tasks),
					lookups: Arc::downgrade(&self.lookups),
					responses: self.responses.clone(),
					snapshot,
					usage: Default::default(),
					capacity: config.endpoint_capacity,
					read_slots: Arc::new(Semaphore::new(config.lookup_concurrency)),
					timeout_reports: self.timeout_reports.clone(),
					watches: self.watches.clone(),
					watch_slots: Arc::new(Semaphore::new(config.watch.poll_concurrency)),
					_capacity_metric: crate::metrics::ENDPOINT_CACHE_CAPACITY
						.add_guard(config.endpoint_capacity as i64),
					_read_capacity_metric: crate::metrics::LOOKUP_READ_CAPACITY
						.add_guard(config.lookup_concurrency as i64),
					_request_capacity_metric: crate::metrics::LOOKUP_REQUEST_CAPACITY
						.add_guard(config.lookup_request_capacity as i64),
				})
			});
			self.snapshot
				.set(runner.snapshot.clone())
				.map_err(|_| anyhow::anyhow!("duplicate topology startup"))?;
			self.timeout_inbox
				.set(runner.inbox.downgrade())
				.map_err(|_| anyhow::anyhow!("duplicate topology startup"))?;
			self.watches.bind(runner.inbox.downgrade())?;
			*runtime = Runtime::Running(runner);
		}

		let result = async {
			let mut ready = self.subscribe()?;
			if registered {
				ready
					.wait_for(|s| s.status != LeaseStatus::Registering)
					.await?;
				ensure!(self.may_admit()?, "topology registration failed");
			} else {
				ready
					.wait_for(|s| s.discovery_round > 0 || s.status != LeaseStatus::Observer)
					.await?;
				ensure!(
					ready.borrow().status == LeaseStatus::Observer && ready.borrow().discovery_ok,
					"initial storage discovery failed"
				);
			}
			Ok(())
		}
		.await;
		if result.is_err() {
			self.shutdown().await?;
		}
		result
	}

	pub fn config(&self) -> &TopologyConfig {
		&self.config
	}

	/// Fresh storage-worker heartbeat evidence for ownership decisions. This bypasses
	/// endpoint hints, coalesces concurrent reads, and never lists stateless workers.
	pub async fn inspect_storage_heartbeat(
		&self,
		worker: &WorkerId,
	) -> Result<crate::heartbeat::Read> {
		let permit = self
			.admission
			.clone()
			.try_acquire_owned()
			.map_err(|error| {
				crate::metrics::LOOKUP_REJECTED.inc();
				tracing::warn!(
					capacity = self.config.lookup_request_capacity,
					"topology heartbeat inspection admission exhausted"
				);
				anyhow::anyhow!(error)
			})?;
		let id = self
			.next_request
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
			.map_err(|_| anyhow::anyhow!("topology request ID exhausted"))?;
		let inbox = self
			.runner
			.lock()
			.await
			.running()
			.context("topology stopped")?
			.inbox
			.clone();
		let (tx, rx) = oneshot::channel();
		{
			let mut responses = self.responses.lock().await;
			responses.insert(id, (ResponseSender::InspectHeartbeat(tx), permit));
			if let Err(error) = inbox.try_send(Event::InspectHeartbeatRequest {
				request_id: id,
				worker: worker.clone(),
			}) {
				responses.remove(&id);
				anyhow::bail!("topology event admission failed: {error}");
			}
		}
		drop(inbox);
		rx.await
			.context("topology stopped during heartbeat inspection")
	}

	pub fn subscribe(&self) -> Result<watch::Receiver<TopologySnapshot>> {
		Ok(self.snapshot.get().context("topology not started")?.clone())
	}

	pub fn snapshot(&self) -> Result<watch::Ref<'_, TopologySnapshot>> {
		let snapshot = self.snapshot.get().context("topology not started")?;
		snapshot.has_changed().context("topology runner stopped")?;

		Ok(snapshot.borrow())
	}

	pub fn may_admit(&self) -> Result<bool> {
		Ok(self.snapshot()?.may_admit(self.env.clock().boot_ms()?))
	}

	/// Admission of new ownership; see `TopologySnapshot::may_take_ownership`.
	pub fn may_take_ownership(&self) -> Result<bool> {
		Ok(self
			.snapshot()?
			.may_take_ownership(self.env.clock().boot_ms()?))
	}

	/// Admission for exactly `worker`, with one boot clock read and one snapshot borrow.
	pub fn permits_worker(&self, worker: &WorkerId) -> Result<bool> {
		let now = self.env.clock().boot_ms()?;
		Ok(self.snapshot()?.permits(worker, now))
	}

	pub fn may_reply(&self) -> Result<bool> {
		Ok(self.snapshot()?.may_reply(self.env.clock().boot_ms()?))
	}

	pub async fn resolve_worker(&self, worker: &WorkerId) -> Result<Option<Worker>> {
		let now = self.env.clock().boot_ms()?;
		{
			let snapshot = self.snapshot()?;
			ensure!(
				!matches!(
					snapshot.status,
					LeaseStatus::Releasing | LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
				),
				"topology stopped"
			);

			if let Some(value) = snapshot.endpoint(worker, now) {
				return Ok(value);
			}
		}
		self.request(
			|request_id| Event::ResolveEndpointRequest {
				request_id,
				worker: worker.clone(),
			},
			ResponseSender::ResolveEndpoint,
		)
		.await
	}

	/// Reports whether `worker`'s lease is live, by an exact-id heartbeat read with its own cache
	/// and admission. It never lists workers. Over capacity, before topology knows store time,
	/// or on a failed read it returns `Unknown` without waiting.
	pub async fn worker_lease(&self, worker: &WorkerId) -> crate::worker_lease::WorkerLease {
		self.worker_lease_instance(worker).await.0
	}

	/// Like `worker_lease`, and also returns the process instance recorded in the heartbeat
	/// that verdict judged. Both come from the same read, so a live verdict can be attributed
	/// to one process without a separate, possibly older, endpoint lookup.
	pub async fn worker_lease_instance(
		&self,
		worker: &WorkerId,
	) -> (
		crate::worker_lease::WorkerLease,
		Option<crate::worker::WorkerInstanceId>,
	) {
		let Ok(now) = self.env.clock().observe() else {
			return (crate::worker_lease::WorkerLease::Unknown, None);
		};
		// A worker discovery lists is a storage worker, so its heartbeat is read in that
		// role first; anything else is assumed stateless.
		let (store_now, first_role) = match self.snapshot() {
			Ok(snapshot) => {
				let store_now = snapshot
					.store_clock
					.as_ref()
					.map(|clock| clock.lower_bound(now.boot_ms));
				let listed = snapshot
					.storage_workers
					.iter()
					.any(|listed| listed.id == *worker);
				let first_role = if listed {
					crate::worker::WorkerRole::Storage
				} else {
					crate::worker::WorkerRole::Stateless
				};
				(store_now, first_role)
			}
			Err(_) => (None, crate::worker::WorkerRole::Stateless),
		};
		self.worker_leases
			.lookup_instance(
				self.store.as_ref(),
				worker,
				now.boot_ms,
				store_now,
				&self.config.lease,
				self.config.lookup_ttl_ms,
				first_role,
			)
			.await
	}

	/// Watches one worker process, `worker` running as `instance`, and returns a handle that
	/// resolves once that process is gone: its lease expired, it released its heartbeat, or a
	/// newer process of the worker claimed it.
	///
	/// This is **for low-urgency cleanup only**, such as unloading the volumes a crashed holder
	/// left open. Detection takes up to `watch.poll_interval_ms` (minutes) for a stateless worker
	/// and one discovery listing for a storage worker. Never use it as the evidence that makes
	/// a fence safe or to decide anything correctness-critical: whatever a caller does when it
	/// fires must stay correct if the process were in fact alive. A maintainer's holder-gone
	/// unload qualifies, because epochs order its recovery claim against any live writer.
	///
	/// Every caller in this worker watching the same process shares one watch, and for a
	/// stateless worker one heartbeat read per interval. Dropping the last handle removes it.
	/// Fails when topology is not running or `watch.capacity` processes are already watched.
	pub async fn watch(
		&self,
		worker: &WorkerId,
		instance: crate::worker::WorkerInstanceId,
	) -> Result<crate::watch::WatchHandle> {
		self.watches
			.watch(crate::topology_machine::watch::WatchedProcess {
				worker_id: worker.clone(),
				instance_id: instance,
			})
			.await
	}

	/// Returns `worker`'s cached lease verdict without any I/O, or `None` when there is no fresh
	/// verdict. Paths that must never wait on a read use this and warm misses elsewhere.
	pub async fn cached_worker_lease(
		&self,
		worker: &WorkerId,
	) -> Option<crate::worker_lease::WorkerLease> {
		let now = self.env.clock().observe().ok()?;
		self.worker_leases.cached(worker, now.boot_ms).await
	}

	/// Waits for invalidation to be published. Does not retry the failed application request.
	/// Hides `worker`'s endpoint after a request to it failed.
	///
	/// This takes no lookup admission. When one connection fails, every request on it
	/// reports the same endpoint, and those reports must not starve the resolution that
	/// replaces the route. Invalidations have their own bounded admission instead, and a
	/// report above it is dropped: the state machine ignores a repeat for an endpoint that
	/// is already hidden, and a later failure reports it again.
	pub async fn invalidate_endpoint(&self, worker: &Worker) -> Result<()> {
		let Ok(permit) = self.invalidations.clone().try_acquire_owned() else {
			crate::metrics::INVALIDATIONS_DROPPED.inc();
			return Ok(());
		};
		self.submit(
			permit,
			|request_id| Event::InvalidateEndpointRequest {
				request_id,
				worker: worker.clone(),
			},
			ResponseSender::InvalidateEndpoint,
		)
		.await?;

		Ok(())
	}

	/// Reports that a stream opening to `worker` timed out. The state machine drops the
	/// cached route only if it was confirmed long ago. This takes no lookup admission and
	/// never waits: when the bounded report window or the inbox is full, the report is dropped.
	pub fn report_endpoint_timeout(&self, worker: &Worker) {
		let capacity = self.config.endpoint_timeout_report_capacity;
		let reserved = self
			.timeout_reports
			.try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
				if n < capacity { Some(n + 1) } else { None }
			})
			.is_ok();
		if !reserved {
			crate::metrics::ENDPOINT_TIMEOUTS
				.with_label_values(&["dropped"])
				.inc();
			return;
		}
		let event = Event::EndpointTimedOut {
			worker: worker.clone(),
		};
		let sent = self
			.timeout_inbox
			.get()
			.and_then(mpsc::WeakSender::upgrade)
			.is_some_and(|inbox| inbox.try_send(event).is_ok());
		if !sent {
			self.timeout_reports.fetch_sub(1, Ordering::AcqRel);
			crate::metrics::ENDPOINT_TIMEOUTS
				.with_label_values(&["dropped"])
				.inc();
		}
	}

	async fn request<T>(
		&self,
		event: impl FnOnce(u64) -> Event,
		response_sender: fn(oneshot::Sender<Result<T, RequestError>>) -> ResponseSender,
	) -> Result<T> {
		let permit = self.admission.clone().try_acquire_owned().map_err(|e| {
			crate::metrics::LOOKUP_REJECTED.inc();
			tracing::warn!(
				capacity = self.config.lookup_request_capacity,
				"topology lookup admission exhausted"
			);
			anyhow::anyhow!("topology lookup admission failed: {e}")
		})?;
		self.submit(permit, event, response_sender).await
	}

	/// Sends an admitted request into the event loop and waits for its response. The
	/// permit is held until the runtime responds, even if the caller is cancelled.
	async fn submit<T>(
		&self,
		permit: tokio::sync::OwnedSemaphorePermit,
		event: impl FnOnce(u64) -> Event,
		response_sender: fn(oneshot::Sender<Result<T, RequestError>>) -> ResponseSender,
	) -> Result<T> {
		let id = self
			.next_request
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
			.map_err(|_| anyhow::anyhow!("topology request ID exhausted"))?;

		let inbox = self
			.runner
			.lock()
			.await
			.running()
			.context("topology stopped")?
			.inbox
			.clone();

		let (tx, rx) = oneshot::channel::<Result<T, RequestError>>();
		{
			let mut responses = self.responses.lock().await;
			responses.insert(id, (response_sender(tx), permit));
			// No cancellation point between registering the response and enqueueing the event.
			if let Err(error) = inbox.try_send(event(id)) {
				responses.remove(&id);
				anyhow::bail!("topology event admission failed: {error}");
			}
		}
		drop(inbox);
		// The runtime retains admission until a response even if this caller is cancelled.
		// This bounds cancelled waiters too, while the state machine coalesces their reads.
		rx.await
			.context("topology stopped before resolving request")?
			.map_err(|e| anyhow::anyhow!("topology request failed: {e:?}"))
	}

	async fn send(&self, event: Event) -> Result<()> {
		let inbox = self
			.runner
			.lock()
			.await
			.running()
			.context("topology stopped")?
			.inbox
			.clone();
		inbox.send(event).await.context("topology runner stopped")
	}

	pub async fn refresh(&self) -> Result<()> {
		let mut snapshot = self.subscribe()?;

		let previous = self.snapshot()?.discovery_round;
		self.send(Event::Refresh).await?;

		let timeout = Duration::from_millis(
			self.config
				.lease
				.refresh_hint_interval_ms
				.saturating_add(self.config.lease.io_timeout_ms)
				.saturating_add(1),
		);
		rivet_env::timeout(
			self.env.clock().as_ref(),
			timeout,
			snapshot.wait_for(|s| {
				s.discovery_round > previous
					|| matches!(
						s.status,
						LeaseStatus::Releasing | LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
					)
			}),
		)
		.await??;

		let current = self.snapshot()?;
		ensure!(
			current.discovery_ok
				&& !matches!(
					current.status,
					LeaseStatus::Releasing | LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
				),
			"topology discovery unavailable"
		);

		Ok(())
	}

	pub async fn refresh_on_miss(&self) -> Result<()> {
		self.refresh().await
	}

	pub async fn storage_workers(&self) -> Result<Vec<crate::worker::Worker>> {
		if self.env.clock().boot_ms()? >= self.snapshot()?.valid_until_boot_ms {
			self.refresh().await?;
		}

		let snapshot = self.snapshot()?;
		ensure!(
			self.env.clock().boot_ms()? < snapshot.valid_until_boot_ms,
			"storage discovery expired"
		);

		Ok(snapshot.storage_workers.clone())
	}

	/// The version to write for persisted `family` data. See `TopologySnapshot::version_for`.
	/// Reads the published snapshot and the boot clock only; it never waits for a listing.
	pub fn version_for(&self, family: crate::versions::VersionFamily) -> Result<u16> {
		let now = self.env.clock().boot_ms()?;
		self.snapshot()?.version_for(family, now)
	}

	/// The version a writer encodes `family` at. See `TopologySnapshot::write_version`.
	/// Reads the published snapshot and the boot clock only; it never waits for a listing.
	pub fn write_version(&self, family: crate::versions::VersionFamily) -> Result<u16> {
		let now = self.env.clock().boot_ms()?;
		self.snapshot()?.write_version(family, now)
	}

	/// The version for a message `peer` decodes and drops. See
	/// `TopologySnapshot::version_for_peer`.
	pub fn version_for_peer(
		&self,
		family: crate::versions::VersionFamily,
		peer: &Worker,
	) -> Result<u16> {
		self.snapshot()?.version_for_peer(family, peer)
	}

	pub async fn begin_drain(&self) -> Result<()> {
		let mut snapshot = self.subscribe()?;
		self.send(Event::BeginDrain).await?;
		snapshot
			.wait_for(|s| {
				s.heartbeat_state == Some(HeartbeatState::Draining)
					|| matches!(
						s.status,
						LeaseStatus::Releasing | LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
					)
			})
			.await?;

		Ok(())
	}

	pub async fn fence(&self, reason: FenceReason) -> Result<()> {
		let mut snapshot = self.subscribe()?;
		self.send(Event::Fence(reason)).await?;
		snapshot
			.wait_for(|s| matches!(s.status, LeaseStatus::Fenced { .. } | LeaseStatus::Stopped))
			.await?;

		Ok(())
	}

	/// Call only after dependents have completed cleanup safely. Stops renewal and
	/// conditionally writes Released; failed or fenced leases expire naturally.
	pub async fn shutdown(&self) -> Result<()> {
		// Keep the handle installed until cleanup finishes so a cancelled shutdown
		// can be resumed by the next caller.
		let mut guard = self.runner.lock().await;

		if !matches!(*guard, Runtime::Running(_)) {
			*guard = Runtime::Stopped;
			self.admission.close();
			return Ok(());
		}
		let Runtime::Running(runner) = &mut *guard else {
			unreachable!()
		};
		self.admission.close();

		let mut snapshot = runner.snapshot.clone();

		let result = async {
			runner.inbox.send(Event::StopRequest).await?;
			snapshot
				.wait_for(|s| matches!(s.status, LeaseStatus::Stopped | LeaseStatus::Fenced { .. }))
				.await?;
			anyhow::Ok(())
		}
		.await;
		self.lookups.shutdown().await;
		self.tasks.shutdown().await;
		self.responses.lock().await.clear();

		let Runtime::Running(runner) = std::mem::replace(&mut *guard, Runtime::Stopped) else {
			unreachable!()
		};
		drop(runner.inbox);
		runner.stopped.await.context("topology runner failed")?;
		result
	}
}

impl crate::versions::WriteVersions for Topology {
	fn write_version(&self, family: crate::versions::VersionFamily) -> Result<u16> {
		Topology::write_version(self, family)
	}
}
