//! Injected persistence and service-lifetime contracts; implementations retain their own runtime handles.
use super::types::*;

use rivet_env::MutationGuard;

/// Maps the named coordinator lease onto the application's single conditional publication object.
#[async_trait::async_trait]
pub trait LeaseStore: Send + Sync + 'static {
	/// Must validate the manifest; malformed/unreachable is Unknown, never Missing.
	async fn read(&self) -> LeaseRead;
	/// Like `read`, but answers `Unchanged` when the store proves the manifest still has
	/// `known_version`, so an idle check transfers no body.
	async fn read_if_changed(&self, known_version: &str) -> ManifestRead;
	/// Preserve all other manifest fields and CAS against attempt.expected's exact version.
	/// The core always reads back, so both unknown and definite conflicts return normally.
	async fn acquire(&self, attempt: &ClaimAttempt, guard: &dyn MutationGuard);
	/// Release only the exact observed tenure/version after the scope's effects are safe.
	async fn release(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		guard: &dyn MutationGuard,
	);
	/// Announce a step-down on the exact observed version, keeping tenure and operator intent.
	async fn step_down(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		step_down_ms: u64,
		guard: &dyn MutationGuard,
	);
	/// Withdraw an announced step-down on the exact observed version.
	async fn withdraw_step_down(
		&self,
		observed: &ObservedLease,
		commit_id: [u8; 16],
		guard: &dyn MutationGuard,
	);
}

use super::{SingletonCoordinator, effect::Effect, event::Event};
use crate::{
	authority::{Authority, DispatchContext},
	runtime::SingletonServices,
};
use rivet_env::{Env, TaskScope};
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

/// Runtime handles for the single local service scope; never persisted as manifest state.
struct ActiveScope {
	tenure: CoordinatorTenure,
	tasks: Arc<TaskScope>,
	/// Set once the scope's tasks and their effects are joined and reconciled.
	joined: Arc<AtomicBool>,
}

/// Dispatches I/O through injected adapters and returns every relevant result through the core.
pub(crate) struct CoordinatorExecutor {
	authority: DispatchContext,
	store: Arc<dyn LeaseStore>,
	services: Arc<dyn SingletonServices>,
	tasks: TaskScope,
	active: parking_lot::Mutex<Option<ActiveScope>>,
	/// How long a revoked scope may take to join before the process exits.
	stop_timeout_ms: u64,
}

impl CoordinatorExecutor {
	pub fn new(
		authority: DispatchContext,
		store: Arc<dyn LeaseStore>,
		services: Arc<dyn SingletonServices>,
		stop_timeout_ms: u64,
	) -> Self {
		let tasks = authority.env.task_scope();
		Self {
			authority,
			store,
			services,
			tasks,
			active: Default::default(),
			stop_timeout_ms,
		}
	}

	pub fn cancel(&self) {
		if let Some(scope) = self.active.lock().as_ref() {
			scope.tasks.cancel();
		}
		self.tasks.cancel();
	}

	pub async fn shutdown(&self) {
		let active = self.active.lock().take();
		if let Some(active) = active {
			active.tasks.shutdown().await;
		}
		self.tasks.shutdown().await;
	}

	fn start_scope(
		&self,
		tenure: CoordinatorTenure,
		inbox: mpsc::Sender<Event>,
		env: &Arc<dyn Env>,
	) {
		let authority = Authority {
			tenure: tenure.clone(),
			runtime: self.authority.clone(),
		};
		if !authority.permits() {
			self.tasks.spawn(async move {
				let _ = inbox
					.send(Event::RelinquishTenure {
						tenure,
						reason: RelinquishReason::ServicesExited,
					})
					.await;
			});
			return;
		}
		let mut active = self.active.lock();
		if let Some(active) = active.as_ref() {
			if active.tenure != tenure {
				tracing::error!(
					"singleton services started for a new tenure while an older one still runs; exiting process"
				);
				let owned_env = env.clone();
				self.tasks
					.spawn(async move { match owned_env.exit(1).await {} });
			}
			return;
		}
		let scope = Arc::new(env.task_scope());
		let services = self.services.clone();
		let watchdog = authority.clone();
		let task = scope.spawn_result(async move {
			if authority.permits() {
				services.run(authority).await
			} else {
				Ok(())
			}
		});
		let joined = Arc::new(AtomicBool::new(false));
		*active = Some(ActiveScope {
			tenure: tenure.clone(),
			tasks: scope,
			joined: joined.clone(),
		});
		drop(active);
		self.watch_revocation(watchdog, joined, env.clone());
		// Observing the task also catches a panic; a silently lost service future must relinquish ownership.
		self.tasks.spawn(async move {
			let _ = task.await;
			let _ = inbox
				.send(Event::RelinquishTenure {
					tenure,
					reason: RelinquishReason::ServicesExited,
				})
				.await;
		});
	}

	/// Bounds the scope by its own clock, independent of the runner: once its authority ends,
	/// the scope must join within the stop timeout, counted from the serving deadline that
	/// ended it, or the process exits. A step-down margin assumes this bound even when the
	/// runner delivers the revocation late. A permit that returns means the core kept the
	/// tenure in a step it ran before the deadline, so the watch starts over.
	fn watch_revocation(&self, authority: Authority, joined: Arc<AtomicBool>, env: Arc<dyn Env>) {
		let stop_timeout_ms = self.stop_timeout_ms;
		self.tasks.spawn(async move {
			loop {
				let deadline_boot_ms = authority.revoked_at().await;
				let now_boot_ms = match env.clock().boot_ms() {
					Ok(now_boot_ms) => now_boot_ms,
					Err(error) => {
						tracing::error!(
							?error,
							"clock unavailable while bounding a revoked singleton scope; exiting process"
						);
						match env.exit(1).await {}
					}
				};
				let revoked_boot_ms =
					deadline_boot_ms.map_or(now_boot_ms, |deadline| deadline.min(now_boot_ms));
				let _ = env
					.clock()
					.sleep_until(revoked_boot_ms.saturating_add(stop_timeout_ms))
					.await;
				if joined.load(Ordering::Acquire) {
					return;
				}
				if authority.permits() {
					continue;
				}
				tracing::error!(
					stop_timeout_ms,
					"revoked singleton scope did not join within the stop timeout; exiting process"
				);
				match env.exit(1).await {}
			}
		});
	}

	fn stop_scope(&self, tenure: CoordinatorTenure, inbox: mpsc::Sender<Event>) {
		let scope = {
			let mut active = self.active.lock();
			if active.as_ref().is_some_and(|scope| scope.tenure != tenure) {
				return;
			}
			active.take()
		};
		if let Some(scope) = &scope {
			scope.tasks.cancel();
		}
		let services = self.services.clone();
		self.tasks.spawn(async move {
			if let Some(scope) = &scope {
				scope.tasks.shutdown().await;
			}
			let effects_safe = services.reconcile_stopped(&tenure).await.is_ok();
			if let Some(scope) = &scope
				&& effects_safe
			{
				scope.joined.store(true, Ordering::Release);
			}
			let _ = inbox
				.send(Event::StopScopeResponse {
					tenure,
					effects_safe,
				})
				.await;
		});
	}
}

impl Executor<SingletonCoordinator> for CoordinatorExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<SingletonCoordinator>) {
		let inbox = ctx.inbox();
		let env = ctx.env();
		match effect {
			Effect::StartScope { tenure } => {
				self.start_scope(tenure, inbox.clone(), env);
				return;
			}
			Effect::StopScopeRequest { tenure } => {
				self.stop_scope(tenure, inbox.clone());
				return;
			}
			Effect::ReadLeaseRequest { .. }
			| Effect::InspectHeartbeatRequest { .. }
			| Effect::AcquireLeaseRequest { .. }
			| Effect::ReleaseLeaseRequest { .. }
			| Effect::StepDownRequest { .. }
			| Effect::WithdrawStepDownRequest { .. }
			| Effect::EntropyRequest { .. } => {}
		}
		let authority = self.authority.clone();
		let store = self.store.clone();
		let inbox = inbox.clone();
		let owned_env = env.clone();
		let deadline = authority.snapshot.borrow().operation_deadline_boot_ms;
		ctx.spawn_in(&self.tasks, async move {
			let Some(deadline) = deadline else {
				return;
			};
			let perform = async {
				let response = match effect {
					Effect::EntropyRequest {
						op,
						min_delay_ms,
						max_delay_ms,
					} => {
						let mut rng = owned_env.rng("singleton-coordinator.retry");
						let attempt_id = *rng.uuid_v4().as_bytes();
						let retry_delay_ms = if min_delay_ms == max_delay_ms {
							min_delay_ms
						} else {
							rng.gen_range(min_delay_ms..max_delay_ms)
						};
						Event::EntropyResponse {
							op,
							attempt_id,
							retry_delay_ms,
						}
					}
					Effect::ReadLeaseRequest { op, known_version } => {
						let result = match known_version {
							Some(version) => store.read_if_changed(&version).await,
							None => ManifestRead::Read(store.read().await),
						};
						Event::ReadLeaseResponse { op, result }
					}
					Effect::InspectHeartbeatRequest { op, owner } => {
						let result = authority
							.topology
							.inspect_storage_heartbeat(&owner.worker_id)
							.await
							.unwrap_or(rivet_topology::heartbeat::Read::Unknown);
						Event::InspectHeartbeatResponse { op, result }
					}
					Effect::AcquireLeaseRequest { op, attempt } => {
						store.acquire(&attempt, &|| authority.may_send(op)).await;
						Event::AcquireLeaseResponse { op }
					}
					Effect::ReleaseLeaseRequest {
						op,
						observed,
						commit_id,
					} => {
						store
							.release(&observed, commit_id, &|| authority.may_send(op))
							.await;
						Event::ReleaseLeaseResponse { op }
					}
					Effect::StepDownRequest {
						op,
						observed,
						commit_id,
						step_down_ms,
					} => {
						store
							.step_down(&observed, commit_id, step_down_ms, &|| {
								authority.may_send(op)
							})
							.await;
						Event::StepDownResponse { op }
					}
					Effect::WithdrawStepDownRequest {
						op,
						observed,
						commit_id,
					} => {
						store
							.withdraw_step_down(&observed, commit_id, &|| authority.may_send(op))
							.await;
						Event::WithdrawStepDownResponse { op }
					}
					Effect::StartScope { .. } | Effect::StopScopeRequest { .. } => {
						unreachable!("scope effects dispatched synchronously")
					}
				};
				let _ = inbox.send(response).await;
			};
			// Cancelling timed-out I/O bounds local resources, never proves a sent PUT did not commit.
			// The core's operation timer independently triggers uncertain-outcome readback.
			rivet_env::select_biased!("expired executor effects cannot issue more work",
			 _ = owned_env.clock().sleep_until(deadline) => {},
			 _ = perform => {},
			);
		});
	}
}
