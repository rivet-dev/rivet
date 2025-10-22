use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_types::runners::Runner;
use universaldb::options::StreamingMode;
use universaldb::utils::{FormalChunkedKey, IsolationLevel::*};

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub runner_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub runners: Vec<Runner>,
}

#[operation]
pub async fn pegboard_runner_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?.to_string();

	let runners = ctx
		.udb()?
		.run(|tx| {
			let dc_name = dc_name.to_string();
			async move {
				let mut runners = Vec::new();

				for runner_id in input.runner_ids.clone() {
					if let Some(runner) = get_inner(&dc_name, &tx, runner_id).await? {
						runners.push(runner);
					}
				}

				Ok(runners)
			}
		})
		.custom_instrument(tracing::info_span!("runner_get_tx"))
		.await?;

	Ok(Output { runners })
}

pub(crate) async fn get_inner(
	dc_name: &str,
	tx: &universaldb::Transaction,
	runner_id: Id,
) -> Result<Option<Runner>> {
	let tx = tx.with_subspace(keys::subspace());

	// TODO: Make this part of the below try join to reduce round trip count
	// Check if runner exists by looking for workflow ID
	if !tx
		.exists(&keys::runner::WorkflowIdKey::new(runner_id), Serializable)
		.await?
	{
		return Ok(None);
	}

	let namespace_id_key = keys::runner::NamespaceIdKey::new(runner_id);
	let name_key = keys::runner::NameKey::new(runner_id);
	let key_key = keys::runner::KeyKey::new(runner_id);
	let version_key = keys::runner::VersionKey::new(runner_id);
	let total_slots_key = keys::runner::TotalSlotsKey::new(runner_id);
	let remaining_slots_key = keys::runner::RemainingSlotsKey::new(runner_id);
	let create_ts_key = keys::runner::CreateTsKey::new(runner_id);
	let connected_ts_key = keys::runner::ConnectedTsKey::new(runner_id);
	let drain_ts_key = keys::runner::DrainTsKey::new(runner_id);
	let stop_ts_key = keys::runner::StopTsKey::new(runner_id);
	let last_ping_ts_key = keys::runner::LastPingTsKey::new(runner_id);
	let last_rtt_key = keys::runner::LastRttKey::new(runner_id);
	let metadata_key = keys::runner::MetadataKey::new(runner_id);
	let metadata_subspace = keys::subspace().subspace(&metadata_key);

	let (
		namespace_id,
		name,
		key,
		version,
		total_slots,
		remaining_slots,
		create_ts,
		connected_ts,
		drain_ts,
		stop_ts,
		last_ping_ts,
		last_rtt,
		metadata_chunks,
	) = tokio::try_join!(
		// NOTE: These are not Serializable because this op is meant for basic information (i.e. data for the
		// API)
		tx.read(&namespace_id_key, Snapshot),
		tx.read(&name_key, Snapshot),
		tx.read(&key_key, Snapshot),
		tx.read(&version_key, Snapshot),
		tx.read(&total_slots_key, Snapshot),
		tx.read(&remaining_slots_key, Snapshot),
		tx.read(&create_ts_key, Snapshot),
		tx.read_opt(&connected_ts_key, Snapshot),
		tx.read_opt(&drain_ts_key, Snapshot),
		tx.read_opt(&stop_ts_key, Snapshot),
		tx.read_opt(&last_ping_ts_key, Snapshot),
		tx.read_opt(&last_rtt_key, Snapshot),
		async {
			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&metadata_subspace).into()
				},
				Snapshot,
			)
			.try_collect::<Vec<_>>()
			.await
			.map_err(Into::into)
		},
	)?;

	let metadata = if metadata_chunks.is_empty() {
		None
	} else {
		Some(metadata_key.combine(metadata_chunks)?.metadata)
	};

	std::result::Result::Ok(Some(Runner {
		runner_id,
		namespace_id,
		datacenter: dc_name.to_string(),
		name,
		key,
		version,
		total_slots,
		remaining_slots,
		create_ts,
		last_connected_ts: connected_ts,
		drain_ts,
		stop_ts,
		last_ping_ts: last_ping_ts.unwrap_or_default(),
		last_rtt: last_rtt.unwrap_or_default(),
		metadata,
	}))
}
