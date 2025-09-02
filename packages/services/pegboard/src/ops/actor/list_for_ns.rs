use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_types::actors::Actor;
use udb_util::{SNAPSHOT, TxnExt};
use universaldb::{self as udb, options::StreamingMode};

use crate::keys;

#[derive(Debug, Default)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub include_destroyed: bool,
	pub created_before: Option<i64>,
	pub limit: usize,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<Actor>,
}

#[operation]
pub async fn pegboard_actor_list_for_ns(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let actors_with_wf_ids = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());
			let mut results = Vec::new();

			if let Some(key) = &input.key {
				let actor_subspace = txs.subspace(&keys::ns::ActorByKeyKey::subspace(
					input.namespace_id,
					input.name.clone(),
					key.clone(),
				));
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					udb_util::end_of_key_range(&txs.pack(
						&keys::ns::ActorByKeyKey::subspace_with_create_ts(
							input.namespace_id,
							input.name.clone(),
							key.clone(),
							created_before,
						),
					))
				} else {
					end
				};

				let mut stream = txs.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					SNAPSHOT,
				);

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, data) = txs.read_entry::<keys::ns::ActorByKeyKey>(&entry)?;

					if !data.is_destroyed || input.include_destroyed {
						results.push((idx_key.actor_id, data.workflow_id));

						if results.len() >= input.limit {
							break;
						}
					}
				}
			} else if input.include_destroyed {
				let actor_subspace = txs.subspace(&keys::ns::AllActorKey::subspace(
					input.namespace_id,
					input.name.clone(),
				));
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					udb_util::end_of_key_range(&txs.pack(
						&keys::ns::AllActorKey::subspace_with_create_ts(
							input.namespace_id,
							input.name.clone(),
							created_before,
						),
					))
				} else {
					end
				};

				let mut stream = txs.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					SNAPSHOT,
				);

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, workflow_id) = txs.read_entry::<keys::ns::AllActorKey>(&entry)?;

					results.push((idx_key.actor_id, workflow_id));

					if results.len() >= input.limit {
						break;
					}
				}
			} else {
				let actor_subspace = txs.subspace(&keys::ns::ActiveActorKey::subspace(
					input.namespace_id,
					input.name.clone(),
				));
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					udb_util::end_of_key_range(&txs.pack(
						&keys::ns::ActiveActorKey::subspace_with_create_ts(
							input.namespace_id,
							input.name.clone(),
							created_before,
						),
					))
				} else {
					end
				};

				let mut stream = txs.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					SNAPSHOT,
				);

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, workflow_id) =
						txs.read_entry::<keys::ns::ActiveActorKey>(&entry)?;

					results.push((idx_key.actor_id, workflow_id));

					if results.len() >= input.limit {
						break;
					}
				}
			}

			Ok(results)
		})
		.custom_instrument(tracing::info_span!("actor_list_tx"))
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
			key: actor_state.key,
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
