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
pub async fn epoxy_replica(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	setup_replica(ctx, input).await?;

	// Main loop
	ctx.repeat(|ctx| {
		async move {
			// Noop for now
			ctx.listen::<Main>().await?;
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

#[signal("epoxy_replica_main")]
pub struct Main {}
