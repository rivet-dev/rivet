//! Core commands use the same registration path and idempotency contract as applications.
use super::KvHandle;
use anyhow::Result;
use rivet_data::generated::depot_kv_command_v1::{KvApply, KvGet, KvGetResponse, Mutation};

/// Read only: retries select another latest committed snapshot.
pub(super) async fn get(kv: KvHandle, request: KvGet) -> Result<KvGetResponse> {
	let value = kv.snapshot().await?.get(&request.key).await?;
	Ok(KvGetResponse {
		value: (value.map(|value| value.to_vec())).map(Into::into),
	})
}

/// Fixed puts and deletes are idempotent; the entire mutation list is one atomic commit.
pub(super) async fn apply(kv: KvHandle, request: KvApply) -> Result<()> {
	let mut txn = kv.begin().await?;
	for mutation in request.mutations {
		match mutation {
			Mutation::Put(put) => txn.put(&put.key, &put.value)?,
			Mutation::Delete(delete) => txn.delete(&delete.key)?,
		}
	}
	kv.commit(txn).await?;
	Ok(())
}
