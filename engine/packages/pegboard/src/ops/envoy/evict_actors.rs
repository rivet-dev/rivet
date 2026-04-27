use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::prelude::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
}

#[operation]
pub async fn pegboard_envoy_evict_actors(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let actors = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let actor_subspace = keys::subspace().subspace(&keys::envoy::ActorKey::subspace(
				input.namespace_id,
				input.envoy_key.clone(),
			));

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&actor_subspace).into()
				},
				Serializable,
			)
			.map(|res| {
				let (key, generation) = tx.read_entry::<keys::envoy::ActorKey>(&res?)?;

				Ok((key.actor_id, generation))
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("envoy_list_actors_tx"))
		.await?;

	// TODO: Parallelize
	for (actor_id, generation) in actors {
		ctx.signal(crate::workflows::actor2::GoingAway { generation })
			.to_workflow::<crate::workflows::actor2::Workflow>()
			.tag("actor_id", actor_id)
			.graceful_not_found()
			.send()
			.await?;
	}

	Ok(())
}
