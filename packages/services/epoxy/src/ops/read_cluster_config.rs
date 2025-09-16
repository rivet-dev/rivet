use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;

use crate::utils;

#[derive(Debug)]
pub struct Input {
	pub replica_id: ReplicaId,
}

#[derive(Debug)]
pub struct Output {
	pub config: protocol::ClusterConfig,
}

#[operation]
pub async fn epoxy_read_cluster_config(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let config = ctx
		.udb()?
		.run(|tx| {
			let replica_id = input.replica_id;
			async move { utils::read_config(&tx, replica_id).await }
		})
		.await?;

	Ok(Output { config })
}
