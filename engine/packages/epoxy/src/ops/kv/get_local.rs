use anyhow::Result;
use epoxy_protocol::protocol::{CachedValue, CommittedValue, ReplicaId};
use gas::prelude::*;
use universaldb::utils::{FormalKey, IsolationLevel::Serializable};

use crate::keys::{self, KvOptimisticCacheKey, KvValueKey, LegacyCommittedValueKey};

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
}

#[operation]
pub async fn epoxy_kv_get_local(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<CommittedValue>> {
	Ok(read_local_value(ctx, input.replica_id, &input.key, false)
		.await?
		.value)
}

#[derive(Debug)]
pub(crate) struct LocalValueRead {
	pub value: Option<CommittedValue>,
	pub cache_value: Option<CachedValue>,
}

/// Reads a committed value from the local replica with dual-read fallback.
///
/// This performs a cascading lookup across storage generations so that values written
/// before the v2 migration remain readable without a full data migration:
///
/// 1. **V2 value** (`EPOXY_V2/replica/{id}/kv/{key}/value`). The current write path.
/// 2. **Legacy committed value** (`EPOXY_V1/replica/{id}/kv/{key}/committed_value`). Written by
///    the original EPaxos protocol. Deserialized as raw bytes with version 0 and mutable=false.
/// 3. **Optimistic cache** (`EPOXY_V2/replica/{id}/kv/{key}/cache`). Only checked when
///    `include_cache` is true. Contains values fetched from remote replicas for the optimistic
///    read path.
///
/// The first path that returns a value wins. This lets the background backfill migrate data
/// at its own pace without blocking reads.
pub(crate) async fn read_local_value(
	ctx: &OperationCtx,
	replica_id: ReplicaId,
	key: &[u8],
	include_cache: bool,
) -> Result<LocalValueRead> {
	ctx.udb()?
		.run(|tx| {
			async move {
				let value_key = KvValueKey::new(key.to_vec());
				let legacy_value_key = LegacyCommittedValueKey::new(key.to_vec());
				let cache_key = KvOptimisticCacheKey::new(key.to_vec());
				let packed_value_key = keys::subspace(replica_id).pack(&value_key);
				let packed_legacy_value_key =
					keys::legacy_subspace(replica_id).pack(&legacy_value_key);
				let packed_cache_key = keys::subspace(replica_id).pack(&cache_key);

				let (local_value, legacy_value, cache_value) = tokio::try_join!(
					tx.get(&packed_value_key, Serializable),
					tx.get(&packed_legacy_value_key, Serializable),
					async {
						if include_cache {
							tx.get(&packed_cache_key, Serializable).await
						} else {
							Ok(None)
						}
					},
				)?;

				// V2 committed value (current write path)
				if let Some(value) = local_value {
					return Ok(LocalValueRead {
						value: Some(value_key.deserialize(&value)?),
						cache_value: None,
					});
				}

				// Legacy committed value (original EPaxos raw bytes)
				if let Some(value) = legacy_value {
					return Ok(LocalValueRead {
						value: Some(CommittedValue {
							value: Some(legacy_value_key.deserialize(&value)?),
							version: 0,
							mutable: false,
						}),
						cache_value: None,
					});
				}

				if let Some(value) = cache_value {
					return Ok(LocalValueRead {
						value: None,
						cache_value: Some(cache_key.deserialize(&value)?),
					});
				}

				Ok(LocalValueRead {
					value: None,
					cache_value: None,
				})
			}
		})
		.custom_instrument(tracing::info_span!("read_local_value_tx"))
		.await
}
