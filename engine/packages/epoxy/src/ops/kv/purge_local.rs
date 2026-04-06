use anyhow::Result;
use epoxy_protocol::protocol::{KvPurgeCacheEntry, ReplicaId};
use gas::prelude::*;
use universaldb::utils::IsolationLevel::Serializable;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub entries: Vec<KvPurgeCacheEntry>,
}

#[operation]
pub async fn epoxy_kv_purge_local(ctx: &OperationCtx, input: &Input) -> Result<()> {
	ctx.udb()?
		.run(|tx| {
			let entries = input.entries.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace(input.replica_id));

				for entry in &entries {
					let cache_key = keys::KvOptimisticCacheKey::new(entry.key.clone());
					if let Some(cached_value) = tx.read_opt(&cache_key, Serializable).await? {
						if cached_value.version <= entry.version {
							tx.delete(&cache_key);
						}
					}
				}

				Ok(())
			}
		})
		.await?;

	Ok(())
}
