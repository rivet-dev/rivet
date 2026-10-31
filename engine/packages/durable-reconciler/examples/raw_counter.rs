//! Run with `cargo run -p rivet-durable-reconciler --example raw_counter`.
//! Uses the in-memory example store; Depot partition storage is not available in this build.
#[path = "support/runtime.rs"]
mod runtime;
use anyhow::{Context, Result};
use bytes::Bytes;
use rivet_durable_reconciler::{
	core::{ReconcileConfig, Reconciler, run_reconciler},
	depot::DurableStore,
};
use rivet_keys::keys::codec::{FormalKey, KeyAddress};
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

#[derive(Clone)]
pub struct Counter<S> {
	pub depot: S,
	pub partition: Bytes,
	pub key: CounterKey,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetCounter {
	pub revision: u64,
	pub value: i64,
}

impl<S: DurableStore> Reconciler for Counter<S> {
	const NAME: &'static str = "raw_counter";

	type State = CounterRecord;

	type Effect = SetCounter;

	async fn load(&self) -> Result<CounterRecord> {
		let bytes = self
			.depot
			.kv_get(&self.partition, &self.key.pack())
			.await?
			.context("missing counter")?;
		self.key.deserialize(bytes.into())
	}

	fn reconcile(state: &CounterRecord) -> Vec<SetCounter> {
		if state.desired.value == state.actual.value {
			return Vec::new();
		}
		vec![SetCounter {
			revision: state.revision,
			value: state.desired.value,
		}]
	}

	async fn execute(&self, effect: SetCounter) -> Result<()> {
		// This write is the effect. The raw implementation owns its concurrency
		// checks and progress save. Repeated or superseded effects become no-ops.
		self.depot
			.kv_mutate(&self.partition, &self.key, |record| {
				let state = record.as_mut().context("missing counter")?;
				if state.revision == effect.revision {
					state.actual.value = effect.value;
					state.revision = state.revision.checked_add(1).context("revision overflow")?;
				}
				Ok(())
			})
			.await
	}
}

/// Same counter as depot_counter, with persistence managed explicitly.
pub async fn example<S: DurableStore>(depot: S) -> Result<()> {
	let rt = runtime::Runtime::new()?;
	let partition = CounterPartition::new(0).pack();
	let key = CounterKey::new(0);
	depot
		.kv_mutate(&partition, &key, |record| {
			record.get_or_insert(CounterRecord {
				revision: 1,
				desired: Desired { value: 0 },
				actual: Actual { value: 0 },
			});
			Ok(())
		})
		.await?;

	let counter = Counter {
		depot: depot.clone(),
		partition: partition.clone(),
		key: key.clone(),
	};

	// Intent survives a stopped loop; each new runner resumes from Depot.
	for target in [10, 20] {
		depot
			.kv_mutate(&partition, &key, |record| {
				let state = record.as_mut().context("missing counter")?;
				state.desired.value = target;
				state.revision = state.revision.checked_add(1).context("revision overflow")?;
				Ok(())
			})
			.await?;
		let cancellation = CancellationToken::new();
		let worker = rt.spawn(run_reconciler(
			rt.env.clone(),
			ReconcileConfig {
				poll_interval: Duration::from_millis(10),
				..Default::default()
			},
			counter.clone(),
			cancellation.clone(),
		));
		let result = rt
			.timeout(Duration::from_secs(5), async {
				loop {
					let state = counter.load().await?;
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
	let state = counter.load().await?;
	assert_eq!(state.desired.value, 20);
	assert_eq!(state.actual.value, 20);
	assert_eq!(state.revision, 5);
	Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<()> {
	let cluster = cluster::TestCluster::start().await?;
	let result = example(cluster.client.clone()).await;
	cluster.close().await?;
	result?;
	tracing::info!("Raw counter converged to 20.");
	Ok(())
}
