//! Fresh local authority gates. Remote receivers still need their own fencing or reconciliation.
use crate::singleton_coordinator_machine::{
	snapshot::Snapshot,
	types::{CoordinatorTenure, OpId, WorkerLeaseView},
};
use rivet_env::Env;
use rivet_topology::{Topology, worker::WorkerIdentity};
use std::sync::Arc;
use tokio::sync::watch;

/// A revocable view shared by every service in one coordinator tenure.
/// Passing this token to a remote system does not make that system enforce it.
#[derive(Clone)]
pub struct Authority {
	pub(crate) tenure: CoordinatorTenure,
	pub(crate) runtime: DispatchContext,
}
impl Authority {
	/// Exact owner identity and generation to bind to target-specific conditional effects.
	pub fn tenure(&self) -> &CoordinatorTenure {
		&self.tenure
	}
	/// Re-read both state machines and the clock at admission and immediately before every effect.
	/// Authority rests on the live worker lease, not on Active participation: a draining owner
	/// keeps serving through its announced step-down until the coordinator revokes it.
	pub fn permits(&self) -> bool {
		self.runtime
			.permits(|snapshot, now| snapshot.permits(&self.tenure, now))
	}
	/// Wait for local revocation. A deadline still applies even when notifications are delayed.
	pub async fn revoked(&self) {
		self.revoked_at().await;
	}

	/// Waits for local revocation and returns the serving deadline it last waited on, which a
	/// late revocation is counted from; None when no deadline was ever known.
	pub(crate) async fn revoked_at(&self) -> Option<u64> {
		let mut coordinator = self.runtime.snapshot.clone();
		let mut last_deadline = None;
		let Ok(mut topology) = self.runtime.topology.subscribe() else {
			return last_deadline;
		};
		loop {
			if !self.permits() {
				return last_deadline;
			}
			let Some(deadline) = coordinator.borrow().serving_deadline_boot_ms() else {
				return last_deadline;
			};
			last_deadline = Some(deadline);
			rivet_env::select_biased!("authority ends before new service work",
			 _ = self.runtime.env.clock().sleep_until(deadline) => {},
			 changed = coordinator.changed() => { if changed.is_err() { return last_deadline; } },
			 changed = topology.changed() => { if changed.is_err() { return last_deadline; } },
			);
		}
	}
}

/// Executor authority includes cleanup while Draining, but never permits an expired worker.
#[derive(Clone)]
pub(crate) struct DispatchContext {
	pub env: Arc<dyn Env>,
	pub topology: Arc<Topology>,
	pub identity: WorkerIdentity,
	pub snapshot: watch::Receiver<Snapshot>,
	/// Shared runner-liveness, channel, and fresh-clock checks for executor dispatch.
	pub guard: rivet_state_machine_util::DispatchGuard<Snapshot>,
}
impl DispatchContext {
	/// Domain lease checks are applied to the guard's one fresh clock observation.
	pub fn permits(&self, pred: impl Fn(&Snapshot, u64) -> bool) -> bool {
		self.guard.permits(|local, now| {
			!local.worker_fenced
				&& local
					.worker_deadline_boot_ms
					.is_some_and(|end| now.boot_ms < end)
				&& self.topology.snapshot().ok().is_some_and(|topology| {
					WorkerLeaseView::from_topology(&topology, &self.identity).live(now.boot_ms)
				}) && pred(local, now.boot_ms)
		})
	}

	pub fn may_send(&self, operation: OpId) -> bool {
		self.permits(|snapshot, now| {
			snapshot.pending_operation == Some(operation)
				&& snapshot
					.operation_deadline_boot_ms
					.is_some_and(|end| now < end)
		})
	}
}
