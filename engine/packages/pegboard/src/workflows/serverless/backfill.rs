//! Backfills runner pool workflows for serverless namespaces.
//!
//! This ensures that any serverless configurations that existed before the
//! runner pool workflow was introduced will have their workflows spawned.

use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::keys;
use crate::workflows::actor_runner_name_selector_backfill::MarkCompleteInput;

pub const BACKFILL_NAME: &str = "serverless";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {}

#[workflow]
pub async fn pegboard_serverless_backfill(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	let backfill_data = ctx.activity(BackfillInput {}).await?;

	// Spawn runner pool workflows for each serverless namespace
	for (namespace_id, runner_name) in backfill_data.runners_to_spawn {
		ctx.workflow(crate::workflows::runner_pool::Input {
			namespace_id,
			runner_name: runner_name.clone(),
		})
		.tag("namespace_id", namespace_id)
		.tag("runner_name", runner_name)
		.unique()
		.dispatch()
		.await?;
	}

	ctx.activity(MarkCompleteInput {
		name: BACKFILL_NAME.to_string(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct BackfillInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillOutput {
	pub runners_to_spawn: Vec<(Id, String)>,
}

/// HACK: Volume is low so we don't bother with chunking - just do the entire
/// backfill in one activity.
#[activity(Backfill)]
pub async fn backfill(ctx: &ActivityCtx, _input: &BackfillInput) -> Result<BackfillOutput> {
	let serverless_data: Vec<rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey> = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let serverless_desired_subspace = keys::subspace().subspace(
				&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::entire_subspace(),
			);

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&serverless_desired_subspace).into()
				},
				// NOTE: This is a snapshot to prevent conflict with updates to this subspace
				Snapshot,
			)
			.map(|res| {
				tx.unpack::<rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey>(res?.key())
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("read_serverless_tx"))
		.await?;

	if serverless_data.is_empty() {
		tracing::info!("no serverless data to backfill");
		return Ok(BackfillOutput {
			runners_to_spawn: Vec::new(),
		});
	}

	tracing::info!(count = serverless_data.len(), "backfilling serverless");

	let runner_configs = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: serverless_data
				.iter()
				.map(|key| (key.namespace_id, key.runner_name.clone()))
				.collect(),
			bypass_cache: true,
		})
		.await?;

	let mut runners_to_spawn = Vec::new();

	for key in &serverless_data {
		if !runner_configs
			.iter()
			.any(|rc| rc.namespace_id == key.namespace_id)
		{
			tracing::debug!(
				namespace_id=?key.namespace_id,
				runner_name=?key.runner_name,
				"runner config not found, likely deleted"
			);
			continue;
		};

		runners_to_spawn.push((key.namespace_id, key.runner_name.clone()));
	}

	Ok(BackfillOutput { runners_to_spawn })
}
