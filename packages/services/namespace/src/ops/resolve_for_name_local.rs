use gas::prelude::*;
use rivet_types::namespaces::Namespace;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys, ops::get_local::get_inner};

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
		.run(|tx| {
			let name = input.name.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());

				let Some(namespace_id) = tx
					.read_opt(&keys::ByNameKey::new(name.clone()), Serializable)
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
