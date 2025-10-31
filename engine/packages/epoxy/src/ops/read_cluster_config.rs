use anyhow::*;
use epoxy_protocol::protocol::{self};
use gas::prelude::*;

use crate::utils;

#[derive(Debug)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	pub config: protocol::ClusterConfig,
}

#[operation]
pub async fn epoxy_read_cluster_config(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let config = ctx
		.udb()?
		.run(|tx| async move { utils::read_config(&tx, ctx.config().epoxy_replica_id()).await })
		.custom_instrument(tracing::info_span!("read_cluster_config_tx"))
		.await?;

	Ok(Output { config })
}
