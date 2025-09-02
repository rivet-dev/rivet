use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::actors::Actor;
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

#[operation]
pub async fn pegboard_actor_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let actors_with_wf_ids = ctx
		.udb()?
		.run(|tx, _mc| async move {
			futures_util::stream::iter(input.actor_ids.clone())
				.map(|actor_id| {
					let tx = tx.clone();
					async move {
						let workflow_id_key = keys::actor::WorkflowIdKey::new(actor_id);

						let workflow_id_entry = tx
							.get(&keys::subspace().pack(&workflow_id_key), SERIALIZABLE)
							.await?;

						let Some(workflow_id_entry) = workflow_id_entry else {
							return Ok(None);
						};

						let workflow_id = workflow_id_key
							.deserialize(&workflow_id_entry)
							.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

						Ok(Some((actor_id, workflow_id)))
					}
				})
				.buffer_unordered(1024)
				.try_filter_map(|x| std::future::ready(Ok(x)))
				.try_collect::<Vec<_>>()
				.await
		})
		.custom_instrument(tracing::info_span!("actor_list_wf_tx"))
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
	let mut actors = Vec::with_capacity(wfs.len());

	for (actor_id, workflow_id) in actors_with_wf_ids {
		let Some(wf) = wfs.iter().find(|wf| wf.workflow_id == workflow_id) else {
			// Actor not found
			continue;
		};

		let actor_state = match wf.parse_state::<Option<crate::workflows::actor::State>>() {
			Ok(Some(s)) => s,
			Ok(None) => {
				// Actor did not initialize state yet
				continue;
			}
			Err(err) => {
				tracing::error!(?actor_id, ?workflow_id, ?err, "failed to parse wf state");
				continue;
			}
		};

		actors.push(Actor {
			actor_id,
			name: actor_state.name.clone(),
			key: actor_state.key.clone().into(),
			namespace_id: actor_state.namespace_id,
			datacenter: dc_name.to_string(),
			runner_name_selector: actor_state.runner_name_selector,
			crash_policy: actor_state.crash_policy,

			create_ts: actor_state.create_ts,
			pending_allocation_ts: actor_state.pending_allocation_ts,
			start_ts: actor_state.start_ts,
			sleep_ts: actor_state.sleep_ts,
			connectable_ts: actor_state.connectable_ts,
			destroy_ts: actor_state.destroy_ts,
		});
	}

	Ok(Output { actors })
}
