use anyhow::*;
use epoxy_protocol::protocol::ReplicaId;
use gas::prelude::*;

use super::read_value;

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
	pub key: Vec<u8>,
}

#[derive(Debug)]
pub struct Output {
	pub value: Option<Vec<u8>>,
	pub version: Option<u64>,
	pub mutable: bool,
}

#[operation]
pub async fn epoxy_kv_get_local(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let committed_value =
		read_value::read_local_value(ctx, input.replica_id, input.key.clone(), false)
			.await?
			.value;

	Ok(Output {
		value: committed_value.as_ref().map(|value| value.value.clone()),
		version: committed_value.as_ref().map(|value| value.version),
		mutable: committed_value
			.as_ref()
			.map(|value| value.mutable)
			.unwrap_or(false),
	})
}
