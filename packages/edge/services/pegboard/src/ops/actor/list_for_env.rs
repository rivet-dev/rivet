use std::collections::HashMap;

use chirp_workflow::prelude::*;
use fdb_util::{FormalKey, SERIALIZABLE, SNAPSHOT};
use foundationdb::{self as fdb, options::StreamingMode};
use futures_util::TryStreamExt;

use crate::keys;

#[derive(Debug, Default)]
pub struct Input {
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub include_destroyed: bool,
	pub cursor: Option<Uuid>,
	pub limit: usize,
}

#[derive(Debug)]
pub struct Output {
	pub actor_ids: Vec<Uuid>,
}

#[operation]
pub async fn pegboard_actor_list_for_env(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let actor_ids = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let actor_subspace =
				keys::subspace().subspace(&keys::env::ActorKey::subspace(input.env_id));
			let (start, end) = actor_subspace.range();

			let end = if let Some(actor_id) = input.cursor {
				let create_ts_key = keys::actor::CreateTsKey::new(actor_id);

				// Get create ts of cursor
				if let Some(entry) = tx
					.get(&keys::subspace().pack(&create_ts_key), SERIALIZABLE)
					.await?
				{
					let create_ts = create_ts_key
						.deserialize(&entry)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					keys::subspace().pack(&keys::env::ActorKey::new(
						input.env_id,
						create_ts,
						actor_id,
					))
				} else {
					end
				}
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
						results.push(actor_key.actor_id);

						if results.len() == input.limit {
							break;
						}
					}
				}
			}

			Ok(results)
		})
		.await?;

	Ok(Output { actor_ids })
}
