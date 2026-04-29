use gas::prelude::*;
use universaldb::utils::IsolationLevel::Serializable;

use crate::{keys, types::SqliteNamespaceConfig};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
}

#[operation]
pub async fn namespace_sqlite_config_get(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<SqliteNamespaceConfig> {
	ctx.udb()?
		.run(|tx| {
			let namespace_id = input.namespace_id;
			async move {
				let tx = tx.with_subspace(keys::subspace());
				Ok(tx
					.read_opt(&keys::sqlite_config_key(namespace_id), Serializable)
					.await?
					.unwrap_or_default())
			}
		})
		.custom_instrument(tracing::info_span!("namespace_sqlite_config_get_tx"))
		.await
}
