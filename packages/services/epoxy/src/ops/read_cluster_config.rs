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
pub async fn read_config(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let config = ctx
		.udb()?
		.run(|tx, _| {
			let replica_id = input.replica_id;
			async move {
				utils::read_config(&tx, replica_id)
					.await
					.map_err(|e: anyhow::Error| universaldb::FdbBindingError::CustomError(e.into()))
			}
		})
		.await?;

	Ok(Output { config })
}
