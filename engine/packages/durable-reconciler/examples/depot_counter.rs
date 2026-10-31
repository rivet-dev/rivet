//! Run with `cargo run -p rivet-durable-reconciler --example depot_counter`.
//! Uses the in-memory example store; Depot partition storage is not available in this build.
#[path = "support/runtime.rs"]
mod runtime;
use anyhow::Result;
use rivet_durable_reconciler::{
	core::ReconcileConfig,
	depot::{DepotReconciler, DepotStateMachine, DurableStore, State},
};
use rivet_keys::keys::codec::KeyAddress;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[cfg(not(test))]
#[path = "support/cluster.rs"]
mod cluster;
#[path = "support/keys.rs"]
mod keys;
#[path = "support/versioned.rs"]
mod versioned;

pub use keys::{CounterKey, CounterPartition};
pub use versioned::{Actual, CounterRecord, Desired};

pub struct CounterReconciler;

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetCounter {
	pub value: i64,
}

pub type Counter = DepotStateMachine<CounterReconciler>;

impl DepotReconciler for CounterReconciler {
	const NAME: &'static str = "counter";

	type Desired = Desired;

	type Actual = Actual;

	type StoredState = CounterRecord;

	type Effect = SetCounter;

	fn from_stored(s: CounterRecord) -> State<Desired, Actual> {
		State {
			revision: s.revision,
			desired: s.desired,
			actual: s.actual,
		}
	}

	fn into_stored(s: State<Desired, Actual>) -> CounterRecord {
		CounterRecord {
			revision: s.revision,
			desired: s.desired,
			actual: s.actual,
		}
	}

	fn reconcile(state: &State<Desired, Actual>) -> Option<SetCounter> {
		(state.desired.value != state.actual.value).then_some(SetCounter {
			value: state.desired.value,
		})
	}

	async fn execute(&self, effect: SetCounter) -> Result<Actual> {
		// The authoritative counter changes when the adapter saves this result.
		Ok(Actual {
			value: effect.value,
		})
	}
}

/// Desired state survives restarting the reconciler.
pub async fn example<S: DurableStore>(depot: S) -> Result<()> {
	let rt = runtime::Runtime::new()?;
	let partition = CounterPartition::new(0).pack();
	Counter::create(
		&depot,
		&partition,
		&CounterKey::new(0),
		Desired { value: 0 },
		Actual { value: 0 },
	)
	.await?;

	// Intent survives a stopped loop; each new runner resumes from Depot.
	for target in [10, 20] {
		Counter::mutate_desired(&depot, &partition, &CounterKey::new(0), |desired| {
			desired.value = target;
			Ok(())
		})
		.await?;
		let cancellation = CancellationToken::new();
		let worker = rt.spawn({
			let depot = depot.clone();
			let partition = partition.clone();
			let cancellation = cancellation.clone();
			let env = rt.env.clone();

			async move {
				Counter::run(
					env,
					&depot,
					&partition,
					&CounterKey::new(0),
					CounterReconciler,
					ReconcileConfig {
						poll_interval: Duration::from_millis(10),
						..Default::default()
					},
					cancellation,
				)
				.await
			}
		});
		let result = rt
			.timeout(Duration::from_secs(5), async {
				loop {
					let state = Counter::read(&depot, &partition, &CounterKey::new(0)).await?;
					if state.actual.value == target {
						return Ok::<_, anyhow::Error>(());
					}
					rt.sleep(Duration::from_millis(10)).await;
				}
			})
			.await;
		cancellation.cancel();
		worker.await??;
		result??;
	}
	assert!(
		!Counter::create(
			&depot,
			&partition,
			&CounterKey::new(0),
			Desired { value: 99 },
			Actual { value: 99 }
		)
		.await?
	);
	let state = Counter::read(&depot, &partition, &CounterKey::new(0)).await?;
	assert_eq!(state.desired.value, 20);
	assert_eq!(state.actual.value, 20);
	assert_eq!(state.revision, 5); // creation + two desired updates + two completions
	Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<()> {
	let cluster = cluster::TestCluster::start().await?;
	let result = example(cluster.client.clone()).await;
	cluster.close().await?;
	result?;
	tracing::info!("Depot counter converged to 20.");
	Ok(())
}
