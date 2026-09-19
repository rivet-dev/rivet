use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_types::actors::Actor;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

/// Maximum number of `ActorByKeyKey` entries a single by-key lookup will examine while searching
/// for a live actor.
///
/// Every actor that loses a key reservation race writes its own index entry under the key before it
/// discovers the conflict and destroys itself, so a key accumulates destroyed entries over time and
/// the live actor is not necessarily the newest entry. Without a cap, a key whose live actor is
/// unreachable makes every lookup read the entire history of that key, which has reached tens of
/// thousands of entries and megabytes of reads per lookup in production.
///
/// Exceeding the cap returns an error rather than an empty list. Callers such as `get_or_create`
/// treat an empty list as "no actor exists" and create a replacement, which writes yet another entry
/// under the key, so a silent truncation would feed the growth it is meant to stop.
const MAX_ACTOR_BY_KEY_SCAN_ENTRIES: usize = 4096;

#[derive(Debug, Default)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub include_destroyed: bool,
	pub created_before: Option<i64>,
	pub limit: usize,
	pub fetch_error: bool,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<Actor>,
}

#[operation]
pub async fn pegboard_actor_list_for_ns(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let actors_with_wf_ids = ctx
		.udb()?
		.txn("pegboard_actor_list_for_ns", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let mut results = Vec::new();

			if let Some(key) = &input.key {
				let actor_subspace = keys::subspace().subspace(&keys::ns::ActorByKeyKey::subspace(
					input.namespace_id,
					input.name.clone(),
					key.clone(),
				));
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					universaldb::utils::end_of_key_range(&tx.pack(
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

				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					Snapshot,
				);

				let mut examined = 0;

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, data) = tx.read_entry::<keys::ns::ActorByKeyKey>(&entry)?;

					if !data.is_destroyed || input.include_destroyed {
						results.push((idx_key.actor_id, data.workflow_id));

						if results.len() >= input.limit {
							break;
						}
					}

					examined += 1;

					// Only bound the destroyed-filtering scan. When destroyed actors are included
					// every entry is pushed, so the limit above already bounds the work.
					if !input.include_destroyed && examined >= MAX_ACTOR_BY_KEY_SCAN_ENTRIES {
						tracing::warn!(
							namespace_id=?input.namespace_id,
							name=%input.name,
							key=%key,
							examined,
							"actor key index scan hit its entry cap, the key likely has a live actor that can no longer be resolved and is accumulating index entries from repeated failed creations",
						);

						return Err(errors::Actor::KeyIndexScanLimitExceeded {
							name: input.name.clone(),
							key: key.clone(),
							limit: MAX_ACTOR_BY_KEY_SCAN_ENTRIES,
						}
						.build());
					}
				}
			} else if input.include_destroyed {
				let actor_subspace = keys::subspace().subspace(&keys::ns::AllActorKey::subspace(
					input.namespace_id,
					input.name.clone(),
				));
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					universaldb::utils::end_of_key_range(&tx.pack(
						&keys::ns::AllActorKey::subspace_with_create_ts(
							input.namespace_id,
							input.name.clone(),
							created_before,
						),
					))
				} else {
					end
				};

				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					Snapshot,
				);

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, workflow_id) = tx.read_entry::<keys::ns::AllActorKey>(&entry)?;

					results.push((idx_key.actor_id, workflow_id));

					if results.len() >= input.limit {
						break;
					}
				}
			} else {
				let actor_subspace = keys::subspace().subspace(
					&keys::ns::ActiveActorKey::subspace(input.namespace_id, input.name.clone()),
				);
				let (start, end) = actor_subspace.range();

				let end = if let Some(created_before) = input.created_before {
					universaldb::utils::end_of_key_range(&tx.pack(
						&keys::ns::ActiveActorKey::subspace_with_create_ts(
							input.namespace_id,
							input.name.clone(),
							created_before,
						),
					))
				} else {
					end
				};

				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: StreamingMode::Iterator,
						reverse: true,
						..(start, end).into()
					},
					// NOTE: Does not have to be serializable because we are listing, stale data does not matter
					Snapshot,
				);

				while let Some(entry) = stream.try_next().await? {
					let (idx_key, workflow_id) =
						tx.read_entry::<keys::ns::ActiveActorKey>(&entry)?;

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
