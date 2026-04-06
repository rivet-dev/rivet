use anyhow::Result;
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::*;
use universaldb::utils::{FormalKey, IsolationLevel::Serializable};

use crate::keys::{self, CommittedValue, KvOptimisticCacheKey, KvValueKey, LegacyCommittedValueKey};

#[derive(Debug)]
pub(crate) struct LocalValueRead {
	pub value: Option<CommittedValue>,
	pub cache_value: Option<CommittedValue>,
}

/// Reads a committed value from the local replica with dual-read fallback.
///
/// This performs a cascading lookup across storage generations so that values written
/// before the v2 migration remain readable without a full data migration:
///
/// 1. **V2 value** (`EPOXY_V2/replica/{id}/kv/{key}/value`). The current write path.
/// 2. **Legacy committed value** (`EPOXY_V1/replica/{id}/kv/{key}/committed_value`). Written by
///    the original EPaxos protocol. Deserialized as raw bytes with version 0 and mutable=false.
/// 3. **Legacy v2-format value** (`EPOXY_V1/replica/{id}/kv/{key}/value`). Written during the
///    intermediate v1-to-v2 transition where the key layout matched v2 but the subspace was
///    still v1.
/// 4. **Optimistic cache** (`EPOXY_V2/replica/{id}/kv/{key}/cache`). Only checked when
///    `include_cache` is true. Contains values fetched from remote replicas for the optimistic
///    read path.
///
/// The first path that returns a value wins. This lets the background backfill migrate data
/// at its own pace without blocking reads.
pub(crate) async fn read_local_value(
	ctx: &OperationCtx,
	replica_id: ReplicaId,
	key: Vec<u8>,
	include_cache: bool,
) -> Result<LocalValueRead> {
	let value_key = KvValueKey::new(key.clone());
	let legacy_value_key = LegacyCommittedValueKey::new(key.clone());
	let legacy_v2_value_key = KvValueKey::new(key.clone());
	let cache_key = KvOptimisticCacheKey::new(key);
	let subspace = keys::subspace(replica_id);
	let legacy_subspace = keys::legacy_subspace(replica_id);
	let packed_value_key = subspace.pack(&value_key);
	let packed_legacy_value_key = legacy_subspace.pack(&legacy_value_key);
	let packed_legacy_v2_value_key = legacy_subspace.pack(&legacy_v2_value_key);
	let packed_cache_key = subspace.pack(&cache_key);

	ctx.udb()?
		.run(|tx| {
			let packed_value_key = packed_value_key.clone();
			let packed_legacy_value_key = packed_legacy_value_key.clone();
			let packed_legacy_v2_value_key = packed_legacy_v2_value_key.clone();
			let packed_cache_key = packed_cache_key.clone();
			let value_key = value_key.clone();
			let legacy_value_key = legacy_value_key.clone();
			let legacy_v2_value_key = legacy_v2_value_key.clone();
			let cache_key = cache_key.clone();

			async move {
				// V2 committed value (current write path)
				if let Some(value) = tx.get(&packed_value_key, Serializable).await? {
					return Ok(LocalValueRead {
						value: Some(value_key.deserialize(&value)?),
						cache_value: None,
					});
				}

				// Legacy committed value (original EPaxos raw bytes)
				if let Some(value) = tx.get(&packed_legacy_value_key, Serializable).await? {
					return Ok(LocalValueRead {
						value: Some(CommittedValue {
							value: legacy_value_key.deserialize(&value)?,
							version: 0,
							mutable: false,
						}),
						cache_value: None,
					});
				}

				// Legacy v2-format value (v1 subspace, v2 key layout)
				if let Some(value) = tx.get(&packed_legacy_v2_value_key, Serializable).await? {
					return Ok(LocalValueRead {
						value: Some(legacy_v2_value_key.deserialize(&value)?),
						cache_value: None,
					});
				}

				let cache_value = if include_cache {
					tx.get(&packed_cache_key, Serializable)
						.await?
						.map(|value| cache_key.deserialize(&value))
						.transpose()?
				} else {
					None
				};

				Ok(LocalValueRead {
					value: None,
					cache_value,
				})
			}
		})
		.custom_instrument(tracing::info_span!("read_local_value_tx"))
		.await
}
