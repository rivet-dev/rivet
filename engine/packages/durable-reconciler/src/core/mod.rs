//! Repeatedly derive work from durable state. Notifications are optional;
//! correctness depends on the state loaded by the reconciler.

use std::{future::Future, time::Duration};

use anyhow::{Context, Result, ensure};
use rivet_env::Env;
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio_util::sync::CancellationToken;

pub struct ReconcileConfig {
	/// Wait after each complete pass, including failed or idle passes. Defaults to 1 second.
	/// The first pass starts immediately; later starts are pass duration + this interval apart.
	pub poll_interval: Duration,
	/// Deadline for each effect, including its completion write. Defaults to 30 seconds.
	/// Expiration drops the future; it cannot undo a remote action or an in-flight DB write.
	pub effect_timeout: Duration,
}

impl Default for ReconcileConfig {
	fn default() -> Self {
		Self {
			poll_interval: Duration::from_secs(1),
			effect_timeout: Duration::from_secs(30),
		}
	}
}

/// Use for custom record layouts or completion writes. For the standard Depot
/// desired/actual record, use [`crate::depot::DepotReconciler`].
pub trait Reconciler: Send + Sync + 'static {
	/// Stable, low-cardinality metric label; never a Rust type name or storage key.
	const NAME: &'static str;
	type State: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send + 'static;
	type Effect: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send + 'static;

	/// Read without side effects and decode deterministically; persist clock or random inputs as
	/// intent instead of reading them here. Failures retry on the next pass. Bound DB
	/// requests separately: the effect timeout excludes loading, though cancellation
	/// can interrupt it.
	fn load(&self) -> impl Future<Output = Result<Self::State>> + Send;

	/// Pure and deterministic: derive pending work only from the loaded snapshot.
	/// Effects in one pass must be independent; dependent work is derived on a later pass.
	fn reconcile(state: &Self::State) -> Vec<Self::Effect>;

	/// Persist progress before returning success. External actions must be idempotent
	/// after partial execution and reject superseded requests: a crash or failed save
	/// can repeat a successful action. Reloading must recognize an ambiguous save
	/// that actually committed.
	///
	/// Progress writes must be conditional and deterministic; persistence callbacks
	/// must not perform external actions or read clocks/randomness. The effect timeout
	/// includes persistence. Yield so cancellation can run; it cannot undo remote work.
	fn execute(&self, effect: Self::Effect) -> impl Future<Output = Result<()>> + Send;
}

pub mod reconciliation_machine;
pub use reconciliation_machine::{ReconcilerState, Reconciliation};

/// Dropping this future cancels the runner and its outstanding effects.
pub async fn run_reconciler<R: Reconciler>(
	env: Arc<dyn Env>,
	config: ReconcileConfig,
	reconciler: R,
	cancellation: CancellationToken,
) -> Result<()> {
	run_reconciler_with_wake(env, config, reconciler, None, cancellation).await
}

/// `run_reconciler` that also starts a pass when `wake` is notified. Writers of the
/// durable record in this process notify it, so a change converges without waiting for
/// `poll_interval`; the poll remains the recovery path for writes from other processes.
pub async fn run_reconciler_with_wake<R: Reconciler>(
	env: Arc<dyn Env>,
	config: ReconcileConfig,
	reconciler: R,
	wake: Option<Arc<tokio::sync::Notify>>,
	cancellation: CancellationToken,
) -> Result<()> {
	ensure!(
		!config.poll_interval.is_zero(),
		"reconcile poll interval must be nonzero"
	);
	ensure!(
		!config.effect_timeout.is_zero(),
		"reconcile effect timeout must be nonzero"
	);
	let (failed, mut failure) = tokio::sync::mpsc::channel(1);
	let tasks = tokio_util::task::TaskTracker::new();
	let cancel = CancellationToken::new();
	let guard = cancel.clone().drop_guard();

	let mut runner = rivet_state_machine_ephemeral::Runner::<Reconciliation<R>>::spawn(
		env,
		ReconcilerState::new(config)?,
		Arc::new(reconciliation_machine::executor::ReconcileExecutor::new(
			reconciler,
			cancel.clone(),
			tasks.clone(),
			failed,
		)),
	);
	let result = rivet_env::select_biased!("cancellation stops reconciliation before more effects run",
		_ = cancellation.cancelled() => Ok(()),
		_ = failure.recv() => Err(anyhow::anyhow!("reconciler work task panicked")),
		_ = async {
			loop {
				if runner.snapshot.borrow_and_update().done { break; }
				let woken = rivet_env::select_biased!("a finished runner wins over a wake",
					changed = runner.snapshot.changed() => { if changed.is_err() { break; } false },
					() = async {
						match &wake {
							Some(wake) => wake.notified().await,
							None => std::future::pending().await,
						}
					} => true,
				);
				if woken && runner.inbox.send(reconciliation_machine::event::Body::Wake).await.is_err() {
					break;
				}
			}
		} => Ok(()),
	);
	drop(guard);
	drop(runner.inbox);
	let stopped = runner
		.stopped
		.await
		.context("reconciler runner stopped unexpectedly");
	tasks.close();
	tasks.wait().await;

	ensure!(failure.try_recv().is_err(), "reconciler work task panicked");
	result.and(stopped)
}
