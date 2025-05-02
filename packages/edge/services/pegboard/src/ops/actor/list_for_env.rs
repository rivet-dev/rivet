use std::collections::HashMap;

use chirp_workflow::prelude::*;
use fdb_util::{FormalKey, SNAPSHOT};
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::TryStreamExt;

use crate::keys;

#[derive(Debug, Default)]
pub struct Input {
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub include_destroyed: bool,
	pub created_before: Option<i64>,
	pub limit: usize,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<ActorEntry>,
}

#[derive(Debug)]
pub struct ActorEntry {
	pub actor_id: Uuid,
	pub create_ts: i64,
}

#[operation]
pub async fn pegboard_actor_list_for_env(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let actors = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let actor_subspace =
				keys::subspace().subspace(&keys::env::ActorKey::subspace(input.env_id));
			let (start, end) = actor_subspace.range();

			let end = if let Some(created_before) = input.created_before {
				keys::subspace().pack(&keys::env::ActorKey::new(
					input.env_id,
					created_before,
					Uuid::nil(),
				))
			} else {
				end
			};

			let mut stream = tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::Iterator,
					reverse: true,
					..(start, end).into()
				},
				// NOTE: Does not have to be serializable because we are listing, stale data does not matter
				SNAPSHOT,
			);
			let mut results = Vec::new();

			while let Some(entry) = stream.try_next().await? {
				let actor_key = keys::subspace()
					.unpack::<keys::env::ActorKey>(entry.key())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
				let data = actor_key
					.deserialize(entry.value())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				if input.include_destroyed || !data.is_destroyed {
					// Compute intersection between ds tags and input tags
					let tags_match = input
						.tags
						.iter()
						.all(|(k, v)| data.tags.iter().any(|(k2, v2)| k == k2 && v == v2));

					if tags_match {
						results.push(ActorEntry {
							actor_id: actor_key.actor_id,
							create_ts: actor_key.create_ts,
						});

						if results.len() == input.limit {
							break;
						}
					}
				}
			}

			Ok(results)
		})
		.custom_instrument(tracing::info_span!("actor_list_tx"))
		.await?;

	Ok(Output { actors })
}
