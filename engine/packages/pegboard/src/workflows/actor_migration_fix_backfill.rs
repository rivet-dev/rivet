use std::time::{Duration, Instant};

use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::*;
use universaldb::prelude::*;

use crate::keys;
use crate::workflows::actor_runner_name_selector_backfill::MarkCompleteInput;

const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);
pub const BACKFILL_NAME: &str = "actor_migration_fix_backfill";
const MAX_ENTRIES: usize = 250;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {}

/// Fixes 3 things:
/// - ActorByKeyKey - sets destroyed and updates workflow id
/// - ActiveActorKey - deletes or updates workflow id
/// - AllActorKey - updates workflow id
#[workflow]
pub async fn pegboard_actor_migration_fix_backfill(
	ctx: &mut WorkflowCtx,
	_input: &Input,
) -> Result<()> {
	ctx.loope(Vec::<u8>::new(), |ctx, last_key| {
		async move {
			let res = ctx
				.activity(BackfillChunkInput {
					last_key: last_key.clone(),
				})
				.await?;

			match res {
				BackfillChunkOutput::Continue { new_last_key } => {
					*last_key = new_last_key;
					Ok(Loop::Continue)
				}
				BackfillChunkOutput::Complete {} => Ok(Loop::Break(())),
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
	Continue { new_last_key: Vec<u8> },
	Complete {},
}

#[activity(BackfillChunk)]
async fn backfill_chunk(
	ctx: &ActivityCtx,
	input: &BackfillChunkInput,
) -> Result<BackfillChunkOutput> {
	let new_last_key = ctx
		.udb()?
		.run(|tx| async move {
			let start = Instant::now();
			let tx = tx.with_subspace(keys::subspace());
			let mut new_last_key = Vec::new();
			let mut count = 0;

			let ns_subspace = keys::subspace().subspace(&(NAMESPACE,));
			let range = ns_subspace.range();

			let range_start = if input.last_key.is_empty() {
				&range.0
			} else {
				&input.last_key
			};
			let range_end = &ns_subspace.range().1;

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(range_start.as_slice(), range_end.as_slice()).into()
				},
				Serializable,
			);

			loop {
				if start.elapsed() > EARLY_TXN_TIMEOUT {
					tracing::warn!("timed out reading ns key entries");
					break;
				}

				let Some(entry) = stream.try_next().await? else {
					new_last_key = Vec::new();
					break;
				};

				new_last_key = [entry.key(), &[0xff]].concat();
				count += 1;

				if let Ok((key, data)) = tx.read_entry::<keys::ns::ActorByKeyKey>(&entry) {
					if !data.is_destroyed {
						let destroy_ts_key = keys::actor::DestroyTsKey::new(key.actor_id);
						let wf_id_key = keys::actor::WorkflowIdKey::new(key.actor_id);
						let (destroyed, workflow_id_entry) = tokio::try_join!(
							tx.exists(&destroy_ts_key, Serializable),
							tx.read_opt(&wf_id_key, Serializable),
						)?;

						if let Some(curr_wf_id) = workflow_id_entry {
							if curr_wf_id != data.workflow_id {
								tx.write(
									&key,
									rivet_data::converted::ActorByKeyKeyData {
										workflow_id: curr_wf_id,
										is_destroyed: destroyed,
									},
								)?;
							}
						}
					}
				} else if let Ok((key, wf_id)) = tx.read_entry::<keys::ns::ActiveActorKey>(&entry) {
					let destroy_ts_key = keys::actor::DestroyTsKey::new(key.actor_id);
					let wf_id_key = keys::actor::WorkflowIdKey::new(key.actor_id);
					let (destroyed, workflow_id_entry) = tokio::try_join!(
						tx.exists(&destroy_ts_key, Serializable),
						tx.read_opt(&wf_id_key, Serializable),
					)?;

					if destroyed {
						tx.delete(&key);
					} else if let Some(curr_wf_id) = workflow_id_entry {
						if curr_wf_id != wf_id {
							tx.write(&key, curr_wf_id)?;
						}
					}
				} else if let Ok((key, wf_id)) = tx.read_entry::<keys::ns::AllActorKey>(&entry) {
					let workflow_id_entry = tx
						.read_opt(&keys::actor::WorkflowIdKey::new(key.actor_id), Serializable)
						.await?;

					if let Some(curr_wf_id) = workflow_id_entry {
						if curr_wf_id != wf_id {
							tx.write(&key, curr_wf_id)?;
						}
					}
				}

				if count > MAX_ENTRIES {
					break;
				}
			}

			Ok(new_last_key)
		})
		.custom_instrument(tracing::info_span!("read_actor_migration_fix_backfill_tx"))
		.await?;

	if new_last_key.is_empty() {
		Ok(BackfillChunkOutput::Complete {})
	} else {
		Ok(BackfillChunkOutput::Continue { new_last_key })
	}
}
