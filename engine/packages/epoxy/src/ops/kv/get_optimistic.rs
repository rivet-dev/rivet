use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::{http_client, keys, utils};

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
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
/// This works by:
/// 1. Attempt to read value from optimistic cache
/// 2. If not in cache, attempt to read value locally
/// 3. If not locally, reach out to any datacenter, then cache & return the first datacenter that has a value
///
/// This means that if the value changes, the value will be inconsistent across all datacenters --
/// even if it has a quorum.
///
/// We cannot use quorum reads for the fanout read because of the constraints of Epaxos.
#[operation]
pub async fn epoxy_kv_get_optimistic(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Try to read locally
	let kv_key = keys::keys::KvValueKey::new(input.key.clone());
	let cache_key = keys::keys::KvOptimisticCacheKey::new(input.key.clone());
	let subspace = keys::subspace(input.replica_id);
	let packed_key = subspace.pack(&kv_key);
	let packed_cache_key = subspace.pack(&cache_key);

	let value = ctx
		.udb()?
		.run(|tx| {
			let packed_key = packed_key.clone();
			let packed_cache_key = packed_cache_key.clone();
			let kv_key = kv_key.clone();
			let cache_key = cache_key.clone();
			async move {
				let (value, cache_value) = tokio::try_join!(
					async {
						let v = tx.get(&packed_key, Serializable).await?;
						if let Some(ref bytes) = v {
							Ok(Some(kv_key.deserialize(bytes)?))
						} else {
							Ok(None)
						}
					},
					async {
						let v = tx.get(&packed_cache_key, Serializable).await?;
						if let Some(ref bytes) = v {
							Ok(Some(cache_key.deserialize(bytes)?))
						} else {
							Ok(None)
						}
					}
				)?;

				Ok(value.or(cache_value))
			}
		})
		.custom_instrument(tracing::info_span!("get_optimistic_tx"))
		.await?;

	if value.is_some() {
		return Ok(Output { value });
	}

	// Request fanout to other datacenters, return first datacenter with any non-none value
	let config = ctx
		.op(crate::ops::read_cluster_config::Input {})
		.await?
		.config;

	let quorum_members: Vec<ReplicaId> = utils::get_quorum_members(&config);

	if quorum_members.len() == 1 {
		return Ok(Output { value: None });
	}

	let responses = http_client::fanout_to_replicas(
		input.replica_id,
		&quorum_members,
		utils::QuorumType::Any,
		|replica_id| {
			let config = config.clone();
			let key = input.key.clone();
			let from_replica_id = input.replica_id;
			async move {
				// Create a KV get request message
				let request = protocol::Request {
					from_replica_id,
					to_replica_id: replica_id,
					kind: protocol::RequestKind::KvGetRequest(protocol::KvGetRequest { key }),
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

	for response in responses {
		if let Some(value) = response {
			// Cache value
			ctx.udb()?
				.run(|tx| {
					let packed_cache_key = packed_cache_key.clone();
					let cache_key = cache_key.clone();
					let value_to_cache = value.clone();

					async move {
						let serialized = cache_key.serialize(value_to_cache)?;
						tx.set(&packed_cache_key, &serialized);
						Ok(())
					}
				})
				.custom_instrument(tracing::info_span!("cache_value_tx"))
				.await?;

			return Ok(Output { value: Some(value) });
		}
	}

	// No value found in any datacenter
	Ok(Output { value: None })
}
