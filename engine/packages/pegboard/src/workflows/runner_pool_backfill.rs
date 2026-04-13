//! Backfills runner pool keys in epoxy.

use std::time::{Duration, Instant};

use epoxy::ops::propose::{Command, CommandKind, Proposal, SetCommand};
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::prelude::*;

use crate::keys;
use crate::workflows::actor_runner_name_selector_backfill::MarkCompleteInput;

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);
pub const BACKFILL_NAME: &str = "epoxy_runner_pools";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {}

#[workflow]
pub async fn pegboard_runner_pool_backfill(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.loope(Vec::<u8>::new(), |ctx, last_key| {
		async move {
			let res = ctx
				.activity(BackfillChunkInput {
					last_key: last_key.clone(),
				})
				.await?;

			match res {
				BackfillOutput::Continue(new_last_key) => {
					*last_key = new_last_key;
					Ok(Loop::Continue)
				}
				BackfillOutput::Complete => Ok(Loop::Break(())),
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
enum BackfillOutput {
	Continue(Vec<u8>),
	Complete,
}

#[activity(BackfillChunk)]
async fn backfill_chunk(ctx: &ActivityCtx, input: &BackfillChunkInput) -> Result<BackfillOutput> {
	let (entries, new_last_key) = ctx
		.udb()?
		.run(|tx| async move {
			let start = Instant::now();
			let tx = tx.with_subspace(namespace::keys::subspace());
			let mut new_last_key = Vec::new();
			let mut entries = Vec::new();

			let runner_config_subspace = namespace::keys::subspace()
				.subspace(&keys::runner_config::DataKey::entire_subspace());
			let range = runner_config_subspace.range();

			let range_start = if input.last_key.is_empty() {
				&range.0
			} else {
				&input.last_key
			};
			let range_end = &runner_config_subspace.range().1;

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(range_start.as_slice(), range_end.as_slice()).into()
				},
				Snapshot,
			);

			loop {
				if start.elapsed() > EARLY_TXN_TIMEOUT {
					tracing::warn!("timed out processing pending actors metrics");
					break;
				}

				let Some(entry) = stream.try_next().await? else {
					new_last_key = Vec::new();
					break;
				};

				new_last_key = [entry.key(), &[0xff]].concat();

				if tx
					.unpack::<keys::runner_config::ProtocolVersionKey>(entry.key())
					.is_ok()
				{
					continue;
				}

				entries.push(tx.read_entry::<keys::runner_config::DataKey>(&entry)?);
			}

			Ok((entries, new_last_key))
		})
		.custom_instrument(tracing::info_span!("read_runner_pools_tx"))
		.await?;

	// Save to epoxy
	futures_util::stream::iter(entries)
		.map(|(key, value)| async move {
			let global_runner_config_key = keys::runner_config::GlobalDataKey::new(
				ctx.config().dc_label(),
				key.namespace_id,
				key.name,
			);

			ctx.op(epoxy::ops::propose::Input {
				proposal: Proposal {
					commands: vec![Command {
						kind: CommandKind::SetCommand(SetCommand {
							key: namespace::keys::subspace().pack(&global_runner_config_key),
							value: Some(global_runner_config_key.serialize(value)?),
						}),
					}],
				},
				purge_cache: true,
				mutable: true,
				target_replicas: None,
			})
			.await?;

			anyhow::Ok(())
		})
		.buffer_unordered(512)
		.try_collect::<()>()
		.await?;

	if new_last_key.is_empty() {
		Ok(BackfillOutput::Complete)
	} else {
		Ok(BackfillOutput::Continue(new_last_key))
	}
}
