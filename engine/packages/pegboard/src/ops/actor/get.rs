use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::actors::Actor;
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_ids: Vec<Id>,
	pub fetch_error: bool,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<Actor>,
}

#[operation]
pub async fn pegboard_actor_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let actors_with_wf_ids = ctx
		.udb()?
		.run(|tx| async move {
			futures_util::stream::iter(input.actor_ids.clone())
				.map(|actor_id| {
					let tx = tx.clone();
					async move {
						let workflow_id_key = keys::actor::WorkflowIdKey::new(actor_id);

						let workflow_id_entry = tx
							.get(&keys::subspace().pack(&workflow_id_key), Serializable)
							.await?;

						let Some(workflow_id_entry) = workflow_id_entry else {
							return Ok(None);
						};

						let workflow_id = workflow_id_key.deserialize(&workflow_id_entry)?;

						Ok(Some((actor_id, workflow_id)))
					}
				})
				.buffer_unordered(1024)
				.try_filter_map(|x| std::future::ready(Ok(x)))
				.try_collect::<Vec<_>>()
				.await
		})
		.custom_instrument(tracing::info_span!("actor_get_tx"))
		.await?;

	let wfs = ctx
		.get_workflows(
			actors_with_wf_ids
				.iter()
				.map(|(_, workflow_id)| *workflow_id)
				.collect(),
		)
		.await?;

	let dc_name = ctx.config().dc_name()?.to_string();

	let actors = super::util::build_actors_from_workflows(
		ctx,
		actors_with_wf_ids,
		wfs,
		&dc_name,
		input.fetch_error,
	)
	.await?;

	Ok(Output { actors })
}
