use anyhow::*;
use epoxy_protocol::protocol;
use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types;

mod setup;

pub use setup::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[workflow]
pub async fn replica(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	setup_replica(ctx, input).await?;

	// Main loop
	ctx.repeat(|ctx| {
		async move {
			// Noop for now
			ctx.listen::<MainSignal>().await?;
			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[signal("epoxy_replica_begin_learning")]
pub struct BeginLearningSignal {
	pub config: types::ClusterConfig,
}

#[signal("main")]
pub struct MainSignal {}
