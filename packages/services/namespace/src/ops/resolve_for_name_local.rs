use gas::prelude::*;
use udb_util::{FormalKey, SERIALIZABLE};
use universaldb as udb;

use crate::{errors, keys, ops::get_local::get_inner, types::Namespace};

#[derive(Debug)]
pub struct Input {
	pub name: String,
}

#[operation]
pub async fn namespace_resolve_for_name_local(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<Namespace>> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	ctx.udb()?
		.run(|tx, _mc| {
			let name = input.name.clone();
			async move {
				let name_idx_key = keys::ByNameKey::new(name.clone());

				let name_idx_entry = tx
					.get(&keys::subspace().pack(&name_idx_key), SERIALIZABLE)
					.await?;

				// Namespace not found
				let Some(name_idx_entry) = name_idx_entry else {
					return Ok(None);
				};

				let namespace_id = name_idx_key
					.deserialize(&name_idx_entry)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

				get_inner(namespace_id, &tx).await
			}
		})
		.custom_instrument(tracing::info_span!("namespace_resolve_for_name_tx"))
		.await
		.map_err(Into::into)
}
