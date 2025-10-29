use anyhow::*;
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub keys: Vec<Vec<u8>>,
}

#[operation]
pub async fn epoxy_kv_purge_local(ctx: &OperationCtx, input: &Input) -> Result<()> {
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace(input.replica_id));

			for key in &input.keys {
				tx.delete(&keys::keys::KvOptimisticCacheKey::new(key.clone()));
			}

			Ok(())
		})
		.await?;

	Ok(())
}
