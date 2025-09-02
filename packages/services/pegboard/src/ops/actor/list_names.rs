use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_key_data::converted::ActorNameKeyData;
use udb_util::{SNAPSHOT, TxnExt};
use universaldb::{self as udb, options::StreamingMode};

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub after_name: Option<String>,
	pub limit: usize,
}

#[derive(Debug)]
pub struct Output {
	pub names: util::serde::FakeMap<String, ActorNameKeyData>,
}

#[operation]
pub async fn pegboard_actor_list_names(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let names = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			let actor_name_subspace =
				txs.subspace(&keys::ns::ActorNameKey::subspace(input.namespace_id));
			let (start, end) = actor_name_subspace.range();

			let start = if let Some(name) = &input.after_name {
				txs.pack(&keys::ns::ActorNameKey::new(
					input.namespace_id,
					name.clone(),
				))
			} else {
				start
			};

			txs.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::WantAll,
					limit: Some(input.limit),
					..(start, end).into()
				},
				// NOTE: This is not SERIALIZABLE to prevent contention with inserting new names
				SNAPSHOT,
			)
			.map(|res| match res {
				Ok(entry) => {
					let (key, metadata) = txs.read_entry::<keys::ns::ActorNameKey>(&entry)?;

					Ok((key.name, metadata))
				}
				Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
			})
			.try_collect::<util::serde::FakeMap<_, _>>()
			.await
		})
		.custom_instrument(tracing::info_span!("actor_list_names_tx"))
		.await?;

	Ok(Output { names })
}
