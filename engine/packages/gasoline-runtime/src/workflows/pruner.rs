use std::sync::Arc;

use anyhow::Result;
use futures_util::FutureExt;
use gas::{db::debug::DatabaseDebug, prelude::*};

const MAX_PRUNES: usize = 1000;

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct PruneState {
	last_key: Option<Vec<u8>>,
	prune_count: usize,
}

#[workflow]
pub async fn gasoline_pruner(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.removed::<Repeat>().await?;

	ctx.v(2)
		.repeat(|ctx| {
			async move {
				ctx.loope(PruneState::default(), |ctx, state| {
					async move {
						let res = ctx
							.activity(PruneChunkInput {
								last_key: state.last_key.clone(),
							})
							.await?;

						match res {
							PruneChunkOutput::Continue {
								new_last_key,
								prune_count,
							} => {
								state.last_key = Some(new_last_key);
								state.prune_count += prune_count;

								Ok(Loop::Continue)
							}
							PruneChunkOutput::Complete { prune_count } => {
								state.prune_count += prune_count;

								Ok(Loop::Break(state.prune_count))
							}
						}
					}
					.boxed()
				})
				.await?;

				ctx.sleep(ctx.config().runtime.gasoline_prune_interval_duration())
					.await?;

				Ok(Loop::<()>::Continue)
			}
			.boxed()
		})
		.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct PruneChunkInput {
	last_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PruneChunkOutput {
	Continue {
		new_last_key: Vec<u8>,
		prune_count: usize,
	},
	Complete {
		prune_count: usize,
	},
}

#[activity(PruneChunk)]
async fn prune_chunk(ctx: &ActivityCtx, input: &PruneChunkInput) -> Result<PruneChunkOutput> {
	// Create new db instance with debug trait
	let db = db::DatabaseKv::new(ctx.config().clone(), ctx.pools().clone()).await?
		as Arc<dyn DatabaseDebug + Send + Sync>;

	// Check if pruning is enabled
	let Some(prune_eligibility_duration) =
		ctx.config().runtime.gasoline_prune_eligibility_duration()
	else {
		return Ok(PruneChunkOutput::Complete { prune_count: 0 });
	};

	let before_ts = util::timestamp::now() - prune_eligibility_duration.as_millis() as i64;
	let (prune_count, new_last_key) = db
		.prune_workflows(before_ts, MAX_PRUNES, input.last_key.as_deref())
		.await?;

	tracing::debug!(%prune_count, "pruned workflows");

	if let Some(new_last_key) = new_last_key {
		Ok(PruneChunkOutput::Continue {
			new_last_key,
			prune_count,
		})
	} else {
		Ok(PruneChunkOutput::Complete { prune_count })
	}
}
