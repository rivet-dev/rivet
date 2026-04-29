use gas::prelude::*;

use crate::{keys, types::SqliteNamespaceConfig};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub config: SqliteNamespaceConfig,
}

#[operation]
pub async fn namespace_sqlite_config_put(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<SqliteNamespaceConfig> {
	ctx.udb()?
		.run(|tx| {
			let namespace_id = input.namespace_id;
			let config = input.config.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(&keys::sqlite_config_key(namespace_id), config.clone())?;
				Ok(config)
			}
		})
		.custom_instrument(tracing::info_span!("namespace_sqlite_config_put_tx"))
		.await
}
