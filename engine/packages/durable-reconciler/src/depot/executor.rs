use super::*;

pub(super) struct Adapter<R, K, S> {
	pub(super) depot: S,
	pub(super) partition: Vec<u8>,
	pub(super) key: K,
	pub(super) reconciler: R,
}

impl<R, K, S> Reconciler for Adapter<R, K, S>
where
	R: DepotReconciler,
	S: DurableStore,
	K: FormalKey<Packed = Bytes> + Send + Sync + 'static,
	K::Codec: ValueCodec<Value = R::StoredState, Input = R::StoredState>,
{
	const NAME: &'static str = R::NAME;

	type State = State<R::Desired, R::Actual>;

	type Effect = (u64, R::Effect);

	async fn load(&self) -> Result<Self::State> {
		let state = DepotStateMachine::<R>::read(&self.depot, &self.partition, &self.key).await?;
		self.reconciler.observe(&state);

		Ok(state)
	}

	fn reconcile(state: &Self::State) -> Vec<Self::Effect> {
		R::reconcile(state)
			.map(|effect| (state.revision, effect))
			.into_iter()
			.collect()
	}

	async fn execute(&self, (revision, effect): Self::Effect) -> Result<()> {
		let actual = self.reconciler.execute(effect).await?;
		let mut completion = crate::metrics::Completion::start(R::NAME);
		let result = self
			.depot
			.kv_mutate(&self.partition, &self.key, |record| {
				let mut state =
					R::from_stored(record.take().context("missing state machine record")?);
				// Recheck against fresh state on every kv_mutate attempt. Never attach
				// an old result to a new desired configuration or a newer actual state.
				let result = if state.revision != revision {
					"stale"
				} else if state.actual == actual {
					"unchanged"
				} else {
					state.actual = actual.clone();
					state.revision = state.revision.checked_add(1).context("revision overflow")?;
					"saved"
				};
				*record = Some(R::into_stored(state));

				Ok(result)
			})
			.await?;
		completion.confirmed(result);

		Ok(())
	}
}
