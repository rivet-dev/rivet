//! A desired/actual record in Depot, reconciled one effect at a time.
//!
//! One stored revision covers both desired and actual state. Write intent through
//! `mutate_desired` or `mutate`; the runner saves actual state after successful execution.
//! Intermediate desired configurations may be coalesced before the next poll.
mod executor;
mod state;
mod store;
use executor::Adapter;
pub use state::State;
pub use store::DurableStore;

use crate::core::{ReconcileConfig, Reconciler, run_reconciler};
use anyhow::{Context, Result};
use bytes::Bytes;
use rivet_env::Env;
use rivet_keys::keys::codec::{FormalKey, ValueCodec};
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Debug, future::Future, hash::Hash, marker::PhantomData, sync::Arc};
use tokio_util::sync::CancellationToken;

/// Intermediate desired states may be coalesced before the next poll. Schema
/// conversions must be pure and preserve the revision and both state fields.
/// External actions belong in `execute`.
pub trait DepotReconciler: Send + Sync + 'static {
	/// Stable, low-cardinality metric label; never a Rust type name or storage key.
	const NAME: &'static str;
	type Desired: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send + Sync + 'static;
	type Actual: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send + Sync + 'static;
	type StoredState: Send;
	type Effect: Clone + Debug + Hash + Eq + Serialize + DeserializeOwned + Send + 'static;

	fn from_stored(state: Self::StoredState) -> State<Self::Desired, Self::Actual>;

	fn into_stored(state: State<Self::Desired, Self::Actual>) -> Self::StoredState;

	/// Must be pure and deterministic. `None` means this snapshot is settled.
	fn reconcile(state: &State<Self::Desired, Self::Actual>) -> Option<Self::Effect>;

	/// Called after each successful loop load, including idle polls, but not by
	/// standalone `read`. Telemetry only; must not mutate state or perform external actions.
	fn observe(&self, _state: &State<Self::Desired, Self::Actual>) {}

	/// The adapter conditionally saves the returned actual state; errors leave it
	/// unchanged. External actions must be idempotent after partial execution and
	/// reject stale commands. Derive any deduplication IDs from durable state.
	///
	/// A failed save or crash can repeat successful work. Each pass reloads first to
	/// recognize saves with ambiguous outcomes. The effect timeout includes saving;
	/// yield so cancellation can run. Cancellation cannot undo remote work.
	fn execute(&self, effect: Self::Effect) -> impl Future<Output = Result<Self::Actual>> + Send;
}

/// Do not delete/recreate records while old effects may still complete.
/// Use a fresh partition/key identity for a new lifecycle.
pub struct DepotStateMachine<R>(PhantomData<fn() -> R>);

impl<R: DepotReconciler> DepotStateMachine<R> {
	/// Creates only if absent; returns whether this call created the record.
	pub async fn create<S: DurableStore, K>(
		depot: &S,
		partition: &[u8],
		key: &K,
		desired: R::Desired,
		actual: R::Actual,
	) -> Result<bool>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
	{
		depot
			.kv_mutate(partition, key, |record| {
				if record.is_some() {
					return Ok(false);
				}
				*record = Some(R::into_stored(State {
					revision: 1,
					desired: desired.clone(),
					actual: actual.clone(),
				}));

				Ok(true)
			})
			.await
	}

	pub async fn read<S: DurableStore, K>(
		depot: &S,
		partition: &[u8],
		key: &K,
	) -> Result<State<R::Desired, R::Actual>>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
	{
		let bytes = depot
			.kv_get(partition, &key.pack())
			.await?
			.context("missing state machine record")?;
		Ok(R::from_stored(key.deserialize(bytes.into())?))
	}

	/// `initial` is used only when absent; creation and mutation share one
	/// conditional write at revision 1. Existing revisions increment only on changes.
	/// Even unchanged values cross the durability boundary, preserving in-flight effects.
	///
	/// The callback may repeat on conflicts: it must be deterministic, without side
	/// effects, clock reads, or randomness. Do not blindly retry unknown write outcomes.
	pub async fn mutate<S: DurableStore, K, O: Send>(
		depot: &S,
		partition: &[u8],
		key: &K,
		initial: Option<(R::Desired, R::Actual)>,
		mutate: impl Fn(&mut R::Desired, &mut R::Actual) -> Result<O> + Send + Sync,
	) -> Result<O>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
	{
		depot
			.kv_mutate(partition, key, |record| {
				let existing = record.is_some();
				let mut state = match record.take() {
					Some(record) => R::from_stored(record),
					None => {
						let (desired, actual) =
							initial.as_ref().context("missing state machine record")?;
						State {
							revision: 1,
							desired: desired.clone(),
							actual: actual.clone(),
						}
					}
				};
				let before = (state.desired.clone(), state.actual.clone());
				let output = mutate(&mut state.desired, &mut state.actual)?;

				if existing && (state.desired != before.0 || state.actual != before.1) {
					state.revision = state.revision.checked_add(1).context("revision overflow")?;
				}
				*record = Some(R::into_stored(state));

				Ok(output)
			})
			.await
	}

	/// Requires an existing record. Shares [`Self::mutate`] retry and durability
	/// semantics; unchanged intent preserves the revision.
	pub async fn mutate_desired<S: DurableStore, K, O: Send>(
		depot: &S,
		partition: &[u8],
		key: &K,
		mutate: impl Fn(&mut R::Desired) -> Result<O> + Send + Sync,
	) -> Result<O>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
	{
		Self::mutate(depot, partition, key, None, move |desired, _| {
			mutate(desired)
		})
		.await
	}

	/// Runs one effect per pass; pending work retries without a limit. Completion
	/// writes save only if the loaded revision still matches, incrementing it only
	/// when actual state changes. Stale completions are discarded and replanned.
	pub async fn run<S: DurableStore, K>(
		env: Arc<dyn Env>,
		depot: &S,
		partition: &[u8],
		key: &K,
		reconciler: R,
		config: ReconcileConfig,
		cancellation: CancellationToken,
	) -> Result<()>
	where
		K: FormalKey<Packed = Bytes> + Clone + Send + Sync + 'static,
		K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
	{
		run_reconciler(
			env,
			config,
			Adapter {
				depot: depot.clone(),
				partition: partition.to_vec(),
				key: key.clone(),
				reconciler,
			},
			cancellation,
		)
		.await
	}
}


