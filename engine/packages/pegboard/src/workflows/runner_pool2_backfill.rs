//! Backfills runner pool 2 workflows for every runner config by-variant entry.

use std::time::{Duration, Instant};

use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::keys::namespace::runner_config::RunnerConfigVariant;
use universaldb::prelude::*;

use crate::keys;
use crate::workflows::actor_runner_name_selector_backfill::MarkCompleteInput;

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);
pub const BACKFILL_NAME: &str = "runner_pool2_backfill";
const MAX_ENTRIES: usize = 50;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {}

#[workflow]
pub async fn pegboard_runner_pool2_backfill(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.loope(Vec::<u8>::new(), |ctx, last_key| {
		async move {
			let res = ctx
				.activity(BackfillChunkInput {
					last_key: last_key.clone(),
				})
				.await?;

			match res {
				BackfillChunkOutput::Continue {
					new_last_key,
					entries,
				} => {
					for (namespace_id, runner_name) in entries {
						ctx.workflow(super::runner_pool::Input {
							namespace_id,
							runner_name: runner_name.clone(),
						})
						.tag("namespace_id", namespace_id)
						.tag("runner_name", &runner_name)
						.unique()
						.dispatch()
						.await?;
					}

					*last_key = new_last_key;
					Ok(Loop::Continue)
				}
				BackfillChunkOutput::Complete { entries } => {
					for (namespace_id, runner_name) in entries {
						ctx.workflow(super::runner_pool::Input {
							namespace_id,
							runner_name: runner_name.clone(),
						})
						.tag("namespace_id", namespace_id)
						.tag("runner_name", &runner_name)
						.unique()
						.dispatch()
						.await?;
					}

					Ok(Loop::Break(()))
				}
			}
		}
		.boxed()
	})
	.await?;

	ctx.activity(MarkCompleteInput {
		name: BACKFILL_NAME.to_string(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct BackfillChunkInput {
	last_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BackfillChunkOutput {
	Continue {
		new_last_key: Vec<u8>,
		entries: Vec<(Id, String)>,
	},
	Complete {
		entries: Vec<(Id, String)>,
	},
}

#[activity(BackfillChunk)]
async fn backfill_chunk(
	ctx: &ActivityCtx,
	input: &BackfillChunkInput,
) -> Result<BackfillChunkOutput> {
	let (entries, new_last_key) = ctx
		.udb()?
		.run(|tx| async move {
			let start = Instant::now();
			let tx = tx.with_subspace(namespace::keys::subspace());
			let mut new_last_key = Vec::new();
			let mut entries = Vec::new();

			let by_variant_subspace = namespace::keys::subspace()
				.subspace(&keys::runner_config::ByVariantKey::entire_subspace());
			let range = by_variant_subspace.range();

			let range_start = if input.last_key.is_empty() {
				&range.0
			} else {
				&input.last_key
			};
			let range_end = &by_variant_subspace.range().1;

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(range_start.as_slice(), range_end.as_slice()).into()
				},
				Snapshot,
			);

			loop {
				if start.elapsed() > EARLY_TXN_TIMEOUT {
					tracing::warn!("timed out reading runner config by-variant entries");
					break;
				}

				let Some(entry) = stream.try_next().await? else {
					new_last_key = Vec::new();
					break;
				};

				new_last_key = [entry.key(), &[0xff]].concat();

				let key = tx.unpack::<keys::runner_config::ByVariantKey>(entry.key())?;
				if let RunnerConfigVariant::Serverless = key.variant {
					entries.push((key.namespace_id, key.name));
				}

				if entries.len() > MAX_ENTRIES {
					break;
				}
			}

			Ok((entries, new_last_key))
		})
		.custom_instrument(tracing::info_span!("read_runner_config_by_variant_tx"))
		.await?;

	if new_last_key.is_empty() {
		Ok(BackfillChunkOutput::Complete { entries })
	} else {
		Ok(BackfillChunkOutput::Continue {
			new_last_key,
			entries,
		})
	}
}
