use anyhow::*;
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::*;
use rivet_api_builder::prelude::*;
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
}

#[derive(Debug)]
pub struct Output {
	pub value: Option<Vec<u8>>,
}

#[operation]
pub async fn epoxy_kv_get_local(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Read from local KV store only
	let kv_key = keys::keys::KvValueKey::new(input.key.clone());
	let subspace = keys::subspace(input.replica_id);
	let packed_key = subspace.pack(&kv_key);

	let value = ctx
		.udb()?
		.run(|tx| {
			let packed_key = packed_key.clone();
			let kv_key = kv_key.clone();
			async move {
				let value = tx.get(&packed_key, Serializable).await?;
				if let Some(v) = value {
					Ok(Some(kv_key.deserialize(&v)?))
				} else {
					Ok(None)
				}
			}
		})
		.custom_instrument(tracing::info_span!("get_local_tx"))
		.await?;

	Ok(Output { value })
}
