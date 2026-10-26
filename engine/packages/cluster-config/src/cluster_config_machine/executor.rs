//! Injected object-store and Config effects; transition policy remains in ClusterConfigStateMachine.
use super::{
	ClusterConfigStateMachine, effect::Effect, event::Event, snapshot::Snapshot, types::OpId,
};
use object_store::ObjectStore;
use parking_lot::Mutex;
use rivet_config::Config;
use rivet_env::{Env, TaskHandle, TaskScope};
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;

/// Runtime resources for one config state machine. At most one local executor task is active.
pub(crate) struct ConfigExecutor {
	store: Arc<dyn ObjectStore>,
	current: Config,
	snapshot: watch::Receiver<Snapshot>,
	alive: Arc<AtomicBool>,
	tasks: TaskScope,
	/// Cancellation handles are runtime resources, not a second source of protocol progress.
	active: Mutex<Option<(OpId, TaskHandle)>>,
}
impl ConfigExecutor {
	pub(crate) fn new(
		store: Arc<dyn ObjectStore>,
		current: Config,
		snapshot: watch::Receiver<Snapshot>,
		alive: Arc<AtomicBool>,
		tasks: TaskScope,
	) -> Self {
		Self {
			store,
			current,
			snapshot,
			alive,
			tasks,
			active: Mutex::new(None),
		}
	}
	pub(crate) fn cancel(&self) {
		self.tasks.cancel();
	}
	pub(crate) async fn shutdown(&self) {
		self.tasks.shutdown().await;
	}
}
fn permitted(
	snapshot: &watch::Receiver<Snapshot>,
	alive: &AtomicBool,
	env: &dyn Env,
	op: OpId,
) -> bool {
	alive.load(Ordering::SeqCst)
		&& env.clock().observe().ok().is_some_and(|now| {
			snapshot
				.borrow()
				.pending
				.is_some_and(|call| call.op == op && now.boot_ms < call.deadline_boot_ms)
		})
}
impl Executor<ClusterConfigStateMachine> for ConfigExecutor {
	fn execute(&self, effect: Effect, ctx: EffectContext<ClusterConfigStateMachine>) {
		let inbox = ctx.inbox();
		let env = ctx.env();
		if let Effect::CancelOperation { op } = effect {
			let mut active = self.active.lock();
			if active.as_ref().is_some_and(|(current, _)| *current == op)
				&& let Some((_, task)) = active.take()
			{
				task.abort();
			}
			return;
		}
		let op = match &effect {
			Effect::ReadConfigRequest { op }
			| Effect::PrepareConfigRequest { op, .. }
			| Effect::WriteConfigRequest { op, .. }
			| Effect::ApplyConfigRequest { op, .. } => *op,
			Effect::CancelOperation { .. } => unreachable!(),
		};
		if !permitted(&self.snapshot, &self.alive, env.as_ref(), op) {
			return;
		}
		// Apply synchronously in dispatch order. Returning its completion through the core still
		// validates correlation; an expired/cancelled async apply can never overwrite a newer apply.
		let (immediate, io) = match effect {
			Effect::PrepareConfigRequest { preparation, .. } => (
				Some(Event::PrepareConfigResponse {
					op,
					result: crate::store::prepare(&self.current, preparation)
						.map_err(|e| e.to_string()),
				}),
				None,
			),
			Effect::ApplyConfigRequest { snapshot, .. } => (
				Some(Event::ApplyConfigResponse {
					op,
					result: crate::store::apply(&self.current, snapshot).map_err(|e| e.to_string()),
				}),
				None,
			),
			Effect::WriteConfigRequest { request, .. } => (None, Some(request)),
			_ => (None, None),
		};
		let env_task = env.clone();
		let snapshot = self.snapshot.clone();
		let alive = self.alive.clone();
		let store = self.store.clone();
		let inbox = inbox.clone();
		let task = ctx.spawn_in(&self.tasks, async move {
			if !permitted(&snapshot, &alive, env_task.as_ref(), op) {
				return;
			}
			let response = if let Some(response) = immediate {
				response
			} else if let Some(request) = io {
				Event::WriteConfigResponse {
					op,
					result: crate::store::write(store.as_ref(), request).await,
				}
			} else {
				Event::ReadConfigResponse {
					op,
					result: crate::store::read(store.as_ref()).await,
				}
			};
			let _ = inbox.send(response).await;
		});
		if let Some((_, previous)) = self.active.lock().replace((op, task)) {
			previous.abort();
		}
	}
}
