//! Explicitly owned coordinator runtime; construction never starts Depot or depends on Depot storage.
use crate::{
	authority::{Authority, DispatchContext},
	singleton_coordinator_machine::{
		SingletonCoordinator,
		event::Event,
		executor::{CoordinatorExecutor, LeaseStore},
		snapshot::Snapshot,
		state::State,
		types::{Config, CoordinatorTenure, WorkerLeaseView},
	},
};
use anyhow::{Context, Result, ensure};
use rivet_env::{Env, TaskScope};
use rivet_state_machine_ephemeral::{Runner, RunnerHandle};
use rivet_topology::{
	Topology,
	worker::{WorkerIdentity, WorkerRole},
};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, watch};

/// The existing registered ClusterSingleton services run together, each recovering its own state.
#[async_trait::async_trait]
pub trait SingletonServices: Send + Sync + 'static {
	/// Own every spawned task within this future or a scope joined by reconcile_stopped.
	/// Depot-dependent services may wait for readiness without blocking leadership startup.
	async fn run(&self, authority: Authority) -> Result<()>;
	/// Called after the run future is cancelled and joined. Prove target-side fencing or safe
	/// reconciliation for already-sent effects. Cancellation or an attached epoch is insufficient.
	async fn reconcile_stopped(&self, tenure: &CoordinatorTenure) -> Result<()>;
}

/// Must be closed before Topology releases its heartbeat. Dropping an unclosed owner fails the
/// process so an abandoned lease cannot stay live through unrelated worker heartbeat renewals.
pub struct Coordinator {
	env: Arc<dyn Env>,
	runner: Option<RunnerHandle<SingletonCoordinator>>,
	executor: Arc<CoordinatorExecutor>,
	watchers: TaskScope,
	alive: Arc<AtomicBool>,
	closed: bool,
}

impl Coordinator {
	/// Start one candidate on a registered storage worker, independent of Depot readiness.
	pub fn start(
		env: Arc<dyn Env>,
		topology: Arc<Topology>,
		lease: Arc<dyn LeaseStore>,
		services: Arc<dyn SingletonServices>,
		config: Config,
	) -> Result<Self> {
		let identity = {
			let snapshot = topology.snapshot()?;
			let registration = snapshot
				.worker_registration
				.as_ref()
				.context("coordinator requires registered worker")?;
			ensure!(
				registration.role == WorkerRole::Storage,
				"only storage workers are coordinator candidates"
			);
			ensure!(
				snapshot.may_admit(env.clock().observe()?.boot_ms),
				"coordinator requires live worker lease"
			);
			registration.identity.clone()
		};
		let stop_timeout_ms = config.stop_timeout_ms;
		let state = State::new(identity.clone(), config)?;
		let alive = Arc::new(AtomicBool::new(true));
		let mut executor = None;
		let runner = Runner::spawn_with_executor(env.clone(), state, |snapshot| {
			let runtime = DispatchContext {
				env: env.clone(),
				topology: topology.clone(),
				identity: identity.clone(),
				guard: rivet_state_machine_util::DispatchGuard::new(
					env.clone(),
					snapshot.clone(),
					alive.clone(),
				),
				snapshot,
			};
			let created = Arc::new(CoordinatorExecutor::new(
				runtime,
				lease,
				services,
				stop_timeout_ms,
			));
			executor = Some(created.clone());
			created
		});
		let watchers = env.task_scope();
		watch_worker(&watchers, &env, &topology, &identity, &runner)?;
		Ok(Self {
			env,
			runner: Some(runner),
			executor: executor.expect("executor constructed synchronously"),
			watchers,
			alive,
			closed: false,
		})
	}

	/// Diagnostic subscription; service authority must use the fresh Authority gate instead.
	pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
		self.runner
			.as_ref()
			.expect("open coordinator")
			.snapshot
			.clone()
	}

	/// A handle that hands the tenure over while the process still serves requests.
	pub fn handover(&self) -> Handover {
		let runner = self.runner.as_ref().expect("open coordinator");
		Handover {
			inbox: runner.inbox.downgrade(),
			snapshot: runner.snapshot.clone(),
		}
	}

	/// Revoke services, establish safe release, resolve uncertain writes, then join owned tasks.
	/// A running tenure stops and releases at once; use `Handover::step_down` first to keep
	/// serving until a successor takes over.
	pub async fn close(mut self) -> Result<()> {
		let runner = self.runner.as_mut().context("coordinator already closed")?;
		runner
			.inbox
			.send(Event::CloseRequest)
			.await
			.context("coordinator runner stopped")?;
		runner
			.snapshot
			.wait_for(|snapshot| snapshot.stopped)
			.await
			.context("coordinator stopped before safe cleanup")?;
		self.alive.store(false, Ordering::SeqCst);
		self.watchers.shutdown().await;
		self.executor.shutdown().await;
		let runner = self.runner.take().unwrap();
		drop(runner.inbox);
		runner
			.stopped
			.await
			.context("coordinator runner failed during join")?;
		self.closed = true;
		Ok(())
	}
}

/// Ends this candidate's participation early in a graceful shutdown. An owner announces its
/// step-down and keeps serving until shortly before the deadline, then releases once its scope
/// joins, so a successor takes over while this process can still answer requests. A weak
/// sender keeps the handle from holding the runner open past `Coordinator::close`.
#[derive(Clone)]
pub struct Handover {
	inbox: mpsc::WeakSender<Event>,
	snapshot: watch::Receiver<Snapshot>,
}

impl Handover {
	/// Returns once this candidate holds no tenure and has no unresolved write. Closing the
	/// coordinator afterwards finds it already stopped.
	pub async fn step_down(mut self) -> Result<()> {
		let inbox = self
			.inbox
			.upgrade()
			.context("coordinator closed before the singleton handover")?;
		inbox
			.send(Event::StopRequest)
			.await
			.context("coordinator runner stopped")?;
		drop(inbox);
		self.snapshot
			.wait_for(|snapshot| snapshot.stopped)
			.await
			.context("coordinator stopped before the singleton handover finished")?;
		Ok(())
	}
}

impl Drop for Coordinator {
	fn drop(&mut self) {
		self.alive.store(false, Ordering::SeqCst);
		self.watchers.cancel();
		self.executor.cancel();
		if !self.closed {
			tracing::error!("singleton coordinator dropped without closing; exiting process");
			let env = self.env.clone();
			self.env
				.spawn(Box::pin(async move { match env.exit(1).await {} }));
		}
	}
}

fn watch_worker(
	tasks: &TaskScope,
	env: &Arc<dyn Env>,
	topology: &Arc<Topology>,
	identity: &WorkerIdentity,
	runner: &RunnerHandle<SingletonCoordinator>,
) -> Result<()> {
	let mut worker = topology.subscribe()?;
	let mut coordinator = runner.snapshot.clone();
	let identity = identity.clone();
	let inbox = runner.inbox.clone();
	let owned_env = env.clone();
	tasks.spawn(async move {
		let mut last_sent: Option<WorkerLeaseView> = None;
		let mut hinted: Option<WorkerIdentity> = None;
		loop {
			// Conservatively fencing this candidate must end its heartbeat lifetime as well. Otherwise
			// a surviving Topology renewal could keep an abandoned coordinator grant live indefinitely.
			if coordinator.borrow_and_update().worker_fenced {
				tracing::error!(
					"singleton coordinator fenced this worker's lease; exiting process so its heartbeat lifetime ends too"
				);
				match owned_env.exit(1).await {}
			}
			let now_boot_ms = match owned_env.clock().boot_ms() {
				Ok(now_boot_ms) => now_boot_ms,
				Err(error) => {
					tracing::error!(?error, "clock unavailable while watching the singleton coordinator's worker; exiting process");
					match owned_env.exit(1).await {}
				}
			};
			let (view, unlisted) = {
				let snapshot = worker.borrow_and_update();
				let view = WorkerLeaseView::from_topology(&snapshot, &identity);
				let watched = coordinator.borrow().watched_owner.clone();
				(view, unlisted_owner(&snapshot, watched, now_boot_ms))
			};

			// Endpoint-cache and lookup bookkeeping share this watch with lease state. Forward
			// only views that changed, so lookup churn does not cost a coordinator step each.
			// Lease expiry is enforced by the coordinator's own alarm, not by these events.
			if last_sent.as_ref() != Some(&view) {
				if inbox
					.send(Event::AuthorityChanged(view.clone()))
					.await
					.is_err()
				{
					return;
				}
				last_sent = Some(view);
			}

			// Discovery already reads every storage heartbeat; a listing without the watched
			// owner is the hint that it is handing over. Report each edge once: unlisted, and
			// listed again.
			if unlisted != hinted {
				let event = match (&unlisted, &hinted) {
					(Some(owner), _) => Event::OwnerUnlisted {
						owner: owner.clone(),
					},
					(None, Some(owner)) => Event::OwnerListed {
						owner: owner.clone(),
					},
					(None, None) => unreachable!("unequal options cannot both be None"),
				};
				if inbox.send(event).await.is_err() {
					return;
				}
				hinted = unlisted;
			}

			rivet_env::select_biased!("worker authority loss wins over another renewal notification",
			 changed = coordinator.changed() => { if changed.is_err() { return; } },
			 changed = worker.changed() => {
			  if changed.is_err() {
			   tracing::error!("topology stopped while the singleton coordinator ran; exiting process");
			   match owned_env.exit(1).await {}
			  }
			 },
			);
		}
	});
	Ok(())
}

/// The watched owner when a still-valid storage listing does not show its process as Active.
/// An expired or absent listing is no evidence, so it hints nothing.
fn unlisted_owner(
	snapshot: &rivet_topology::TopologySnapshot,
	watched: Option<WorkerIdentity>,
	now_boot_ms: u64,
) -> Option<WorkerIdentity> {
	let owner = watched?;
	if now_boot_ms >= snapshot.valid_until_boot_ms {
		return None;
	}
	let listed = snapshot
		.storage_workers
		.iter()
		.any(|worker| worker.id == owner.worker_id && worker.instance_id == owner.instance_id);
	if listed { None } else { Some(owner) }
}
