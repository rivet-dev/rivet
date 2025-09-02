use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use udb_util::{FormalKey, SERIALIZABLE};
use universaldb as udb;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_ids: Vec<Id>,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<Actor>,
}

#[derive(Debug)]
pub struct Actor {
	pub actor_id: Id,
	pub runner_id: Id,
	pub is_connectable: bool,
}

// TODO: Add cache (remember to purge cache when runner changes)
#[operation]
pub async fn pegboard_actor_get_runner(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let actors = ctx
		.udb()?
		.run(|tx, _mc| async move {
			futures_util::stream::iter(input.actor_ids.clone())
				.map(|actor_id| {
					let tx = tx.clone();
					async move {
						let runner_id_key = keys::actor::RunnerIdKey::new(actor_id);
						let connectable_key = keys::actor::ConnectableKey::new(actor_id);

						let (runner_id_entry, connectable_entry) = tokio::try_join!(
							tx.get(&keys::subspace().pack(&runner_id_key), SERIALIZABLE),
							tx.get(&keys::subspace().pack(&connectable_key), SERIALIZABLE),
						)?;

						let Some(runner_id_entry) = runner_id_entry else {
							return Ok(None);
						};

						let runner_id = runner_id_key
							.deserialize(&runner_id_entry)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						Ok(Some(Actor {
							actor_id,
							runner_id,
							is_connectable: connectable_entry.is_some(),
						}))
					}
				})
				.buffer_unordered(1024)
				.try_filter_map(|x| std::future::ready(Ok(x)))
				.try_collect::<Vec<_>>()
				.await
		})
		.custom_instrument(tracing::info_span!("actor_get_runner_tx"))
		.await?;

	Ok(Output { actors })
}
