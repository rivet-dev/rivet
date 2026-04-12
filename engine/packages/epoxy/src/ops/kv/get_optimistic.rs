use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use universaldb::prelude::*;

use crate::{
	http_client,
	keys::{self, CommittedValue},
	utils,
};

use super::get_local::read_local_value;

// TODO: Allow multiple keys
#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
	pub caching_behavior: protocol::CachingBehavior,
	/// Optional active-replica scope for this proposal.
	///
	/// Epoxy only validates that the supplied replicas are active and include the local
	/// replica. Callers are responsible for ensuring a given key stays on a stable scope over
	/// time, or that any scope change is handled as an explicit reconfiguration at a higher
	/// layer.
	pub target_replicas: Option<Vec<ReplicaId>>,
	// Whether or not to write an empty value into cache if it did not exist on read.
	pub save_empty: bool,
}

#[derive(Debug)]
pub struct Output {
	pub value: Option<Vec<u8>>,
}

/// WARNING: Do not use this method unless you know for certain that your value will not change
/// after it has been set.
///
/// WARNING: This will cause a lot of overhead if requested frequently without ever resolving a
/// value, since this fans out to all datacenters to attempt to find a datacenter with a value.
///
/// WARNING: This will incorrectly return `None` in the rare case that all of the nodes that have
/// committed the value are offline.
///
/// This reads committed values from the v2 `kv/{key}/value` path, falls back to the legacy
/// `kv/{key}/committed_value` and legacy `kv/{key}/value` paths, then checks the v2
/// `kv/{key}/cache` path for remote values.
///
/// This works by:
/// 1. Attempt to read the v2 committed value locally
/// 2. If not found, fall back to the legacy committed-value path
/// 3. If still not found, fall back to the legacy value path
/// 4. If still not found, check the v2 optimistic cache
/// 5. If not in cache, reach out to any datacenter, then cache and return the first datacenter that has a value
///
/// This means that if the value changes, the value will be inconsistent across all datacenters --
/// even if it has a quorum.
///
/// We cannot use quorum reads for the fanout read because the optimistic path is intentionally a
/// best-effort lookup.
#[operation]
pub async fn epoxy_kv_get_optimistic(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let local_read = read_local_value(
		ctx,
		input.replica_id,
		&input.key,
		input.caching_behavior == protocol::CachingBehavior::Optimistic,
	)
	.await?;
	if let Some(value) = local_read.value.or(local_read.cache_value) {
		return Ok(Output {
			value: Some(value.value),
		});
	}

	// Request fanout to other datacenters, return first datacenter with any non-none value
	let config = ctx
		.op(crate::ops::read_cluster_config::Input {})
		.await?
		.config;

	let replica_id = ctx.config().epoxy_replica_id();
	let quorum_members = utils::resolve_active_quorum_members(
		&config,
		replica_id,
		input.target_replicas.as_deref(),
	)?;

	if quorum_members.len() == 1 {
		return Ok(Output { value: None });
	}

	let responses = http_client::fanout_to_replicas(
		input.replica_id,
		&quorum_members,
		utils::QuorumType::Any,
		|replica_id| {
			let ctx = ctx.clone();
			let config = config.clone();
			let key = input.key.clone();
			let from_replica_id = input.replica_id;
			async move {
				// Create a KV get request message
				let request = protocol::Request {
					from_replica_id,
					to_replica_id: replica_id,
					kind: protocol::RequestKind::KvGetRequest(protocol::KvGetRequest {
						key,
						caching_behavior: input.caching_behavior.clone(),
					}),
				};

				// Send the message and extract the KV response
				let response =
					http_client::send_message(&ApiCtx::new_from_operation(&ctx)?, &config, request)
						.await?;

				match response.kind {
					protocol::ResponseKind::KvGetResponse(kv_response) => Ok(kv_response.value),
					_ => bail!("unexpected response type for KV get request"),
				}
			}
		},
	)
	.await?;

	// Should only have 1 response
	if let Some(value) = responses.into_iter().flatten().next() {
		let value = CommittedValue {
			value: value.value,
			version: value.version,
			mutable: value.mutable,
		};

		if input.caching_behavior == protocol::CachingBehavior::Optimistic {
			cache_fanout_value(ctx, input.replica_id, &input.key, &value).await?;
		}

		return Ok(Output {
			value: Some(value.value),
		});
	} else if input.save_empty {
		cache_empty_value(ctx, input.replica_id, &input.key).await?;
	}

	// No value found in any datacenter
	Ok(Output { value: None })
}

async fn cache_fanout_value(
	ctx: &OperationCtx,
	replica_id: ReplicaId,
	key: &[u8],
	value_to_cache: &CommittedValue,
) -> Result<()> {
	ctx.udb()?
		.run(|tx| {
			async move {
				let tx = tx.with_subspace(keys::subspace(replica_id));
				let cache_key = keys::KvOptimisticCacheKey::new(key.to_vec());

				// Skip caching if the existing cache entry is strictly newer. This
				// prevents a slow fanout response from overwriting a fresher cache entry
				// written by a concurrent request.
				if let Some(existing_cache) = tx.read_opt(&cache_key, Serializable).await? {
					if existing_cache.version > value_to_cache.version {
						return Ok(());
					}
				}

				tx.write(&cache_key, value_to_cache.clone())?;
				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("cache_value_tx"))
		.await
}

async fn cache_empty_value(ctx: &OperationCtx, replica_id: ReplicaId, key: &[u8]) -> Result<()> {
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace(replica_id));
			let cache_key = keys::KvOptimisticCacheKey::new(key.to_vec());

			if tx.exists(&cache_key, Serializable).await? {
				return Ok(());
			}

			tx.write(
				&cache_key,
				CommittedValue {
					value: Vec::new(),
					version: 0,
					// TODO: What should this be set to?
					mutable: true,
				},
			)?;
			Ok(())
		})
		.custom_instrument(tracing::info_span!("cache_empty_value_tx"))
		.await
}
