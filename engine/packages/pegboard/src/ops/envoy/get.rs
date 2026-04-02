use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_types::envoys::Envoy;
use universaldb::options::StreamingMode;
use universaldb::utils::{FormalChunkedKey, IsolationLevel::*};

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub envoys: Vec<Envoy>,
}

#[operation]
pub async fn pegboard_envoy_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?;

	let envoys = ctx
		.udb()?
		.run(|tx| {
			let dc_name = dc_name.to_string();
			async move {
				let mut envoys = Vec::new();

				for envoy_key in &input.envoy_keys {
					if let Some(envoy) =
						get_inner(&dc_name, &tx, input.namespace_id, envoy_key).await?
					{
						envoys.push(envoy);
					}
				}

				Ok(envoys)
			}
		})
		.custom_instrument(tracing::info_span!("envoy_get_tx"))
		.await?;

	Ok(Output { envoys })
}

pub(crate) async fn get_inner(
	dc_name: &str,
	tx: &universaldb::Transaction,
	namespace_id: Id,
	envoy_key: &str,
) -> Result<Option<Envoy>> {
	let tx = tx.with_subspace(keys::subspace());

	// TODO: Make this part of the below try join to reduce round trip count
	// Check if envoy exists by looking for workflow ID
	if !tx
		.exists(
			&keys::envoy::PoolNameKey::new(namespace_id, envoy_key.to_string()),
			Serializable,
		)
		.await?
	{
		return Ok(None);
	}

	let pool_name_key = keys::envoy::PoolNameKey::new(namespace_id, envoy_key.to_string());
	let version_key = keys::envoy::VersionKey::new(namespace_id, envoy_key.to_string());
	let slots_key = keys::envoy::SlotsKey::new(namespace_id, envoy_key.to_string());
	let create_ts_key = keys::envoy::CreateTsKey::new(namespace_id, envoy_key.to_string());
	let connected_ts_key = keys::envoy::ConnectedTsKey::new(namespace_id, envoy_key.to_string());
	let stop_ts_key = keys::envoy::StopTsKey::new(namespace_id, envoy_key.to_string());
	let last_ping_ts_key = keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.to_string());
	let last_rtt_key = keys::envoy::LastRttKey::new(namespace_id, envoy_key.to_string());
	let metadata_key = keys::envoy::MetadataKey::new(namespace_id, envoy_key.to_string());
	let metadata_subspace = keys::subspace().subspace(&metadata_key);

	let (
		pool_name,
		version,
		slots,
		create_ts,
		connected_ts,
		stop_ts,
		last_ping_ts,
		last_rtt,
		metadata_chunks,
	) = tokio::try_join!(
		// NOTE: These are not Serializable because this op is meant for basic information (i.e. data for the
		// API)
		tx.read(&pool_name_key, Snapshot),
		tx.read(&version_key, Snapshot),
		tx.read(&slots_key, Snapshot),
		tx.read(&create_ts_key, Snapshot),
		tx.read_opt(&connected_ts_key, Snapshot),
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

	Ok(Some(Envoy {
		envoy_key: envoy_key.to_string(),
		namespace_id: namespace_id,
		datacenter: dc_name.to_string(),
		pool_name,
		version,
		slots: slots.try_into()?,
		create_ts,
		last_connected_ts: connected_ts,
		stop_ts,
		last_ping_ts: last_ping_ts.unwrap_or_default(),
		last_rtt: last_rtt.unwrap_or_default(),
		metadata,
	}))
}
