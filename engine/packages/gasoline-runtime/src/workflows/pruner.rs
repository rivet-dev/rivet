use std::sync::Arc;

use anyhow::Result;
use futures_util::FutureExt;
use gas::{db::debug::DatabaseDebug, prelude::*};

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[workflow]
pub async fn gasoline_pruner(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.repeat(|ctx| {
		async move {
			ctx.activity(PruneInput {}).await?;

			ctx.sleep(ctx.config().runtime.gasoline.prune_interval_duration())
				.await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct PruneInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PruneOutput {
	prune_count: usize,
}

#[activity(Prune)]
async fn prune(ctx: &ActivityCtx, _input: &PruneInput) -> Result<PruneOutput> {
	// Create new db instance with debug trait
	let db = db::DatabaseKv::new(ctx.config().clone(), ctx.pools().clone()).await?
		as Arc<dyn DatabaseDebug + Send + Sync>;

	// Check if pruning is enabled
	let Some(prune_eligibility_duration) =
		ctx.config().runtime.gasoline.prune_eligibility_duration()
	else {
		return Ok(PruneOutput { prune_count: 0 });
	};

	let before_ts = util::timestamp::now() - prune_eligibility_duration.as_millis() as i64;
	let prune_count = db.prune_workflows(before_ts).await?;

	tracing::debug!(%prune_count, "pruned workflows");

	Ok(PruneOutput { prune_count })
}
