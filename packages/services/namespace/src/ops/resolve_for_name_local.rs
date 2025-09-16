use gas::prelude::*;
use udb_util::{SERIALIZABLE, TxnExt};

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
				let txs = tx.subspace(keys::subspace());

				let Some(namespace_id) = txs
					.read_opt(&keys::ByNameKey::new(name.clone()), SERIALIZABLE)
					.await?
				else {
					// Namespace not found
					return Ok(None);
				};

				get_inner(namespace_id, &tx).await
			}
		})
		.custom_instrument(tracing::info_span!("namespace_resolve_for_name_tx"))
		.await
		.map_err(Into::into)
}
