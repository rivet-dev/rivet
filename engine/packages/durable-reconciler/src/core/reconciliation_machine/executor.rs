use super::{Reconciliation, effect::Effect, event::Body};
use crate::core::Reconciler;
use futures_util::FutureExt;
use rivet_state_machine_ephemeral::{EffectContext, Executor};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub(in crate::core) struct ReconcileExecutor<R> {
	reconciler: Arc<R>,
	cancel: CancellationToken,
	tasks: TaskTracker,
	failed: mpsc::Sender<()>,
	current: parking_lot::Mutex<Option<CancellationToken>>,
}

impl<R> ReconcileExecutor<R> {
	pub(in crate::core) fn new(
		reconciler: R,
		cancel: CancellationToken,
		tasks: TaskTracker,
		failed: mpsc::Sender<()>,
	) -> Self {
		Self {
			reconciler: Arc::new(reconciler),
			cancel,
			tasks,
			failed,
			current: parking_lot::Mutex::new(None),
		}
	}
}

impl<R: Reconciler> Executor<Reconciliation<R>> for ReconcileExecutor<R> {
	fn execute(&self, effect: Effect<R::Effect>, ctx: EffectContext<Reconciliation<R>>) {
		if let Effect::LoadedMetric { success } = effect {
			crate::metrics::loaded(R::NAME, success);
			return;
		}

		if let Some(previous) = self.current.lock().take() {
			previous.cancel();
		}

		if matches!(effect, Effect::CancelWork) {
			return;
		}

		let cancel = self.cancel.child_token();
		*self.current.lock() = Some(cancel.clone());
		let reconciler = self.reconciler.clone();
		let inbox = ctx.inbox().clone();
		let work = async move {
			let body = match effect {
				Effect::Load(op) => Body::Loaded {
					op,
					result: reconciler.load().await.map_err(|e| e.to_string()),
				},
				Effect::Execute(op, effect) => Body::Completed {
					op,
					result: reconciler.execute(effect).await.map_err(|e| e.to_string()),
				},
				_ => return,
			};

			match &body {
				Body::Loaded {
					result: Err(error), ..
				} => {
					tracing::warn!(machine = R::NAME, %error, "reconcile load failed; retrying on next pass");
				}
				Body::Completed {
					result: Err(error), ..
				} => {
					tracing::warn!(machine = R::NAME, %error, "reconcile effect failed; retrying on next pass");
				}
				_ => (),
			}

			let _ = inbox.send(body).await;
		};

		let failed = self.failed.clone();
		ctx.spawn(self.tasks.track_future(async move {
			let result = std::panic::AssertUnwindSafe(async move {
				rivet_env::select_biased!("cancelled reconciliation cannot deliver a completion",
					_ = cancel.cancelled() => (),
					_ = work => (),
				);
			})
			.catch_unwind()
			.await;
			if result.is_err() {
				let _ = failed.try_send(());
			}
		}));
	}
}
