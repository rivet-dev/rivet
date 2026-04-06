use anyhow::*;
use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types;

mod setup;

pub use setup::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[workflow]
pub async fn epoxy_replica_v2(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	setup_replica(ctx, input).await?;

	// Main loop
	ctx.repeat(|ctx| {
		async move {
			let sig = ctx.listen::<BeginLearning>().await?;
			setup::begin_learning(ctx, &sig).await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[signal("epoxy_replica_begin_learning")]
pub struct BeginLearning {
	pub config: types::ClusterConfig,
}
