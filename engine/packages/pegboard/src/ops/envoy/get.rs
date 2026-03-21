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
	pub envoy_key: String,
}

#[operation]
pub async fn pegboard_envoy_get(ctx: &OperationCtx, input: &Input) -> Result<Option<Envoy>> {
	let dc_name = ctx.config().dc_name()?;

	ctx.udb()?
		.run(|tx| {
			async move {
				let tx = tx.with_subspace(keys::subspace());

				// TODO: Make this part of the below try join to reduce round trip count
				// Check if envoy exists by looking for workflow ID
				if !tx
					.exists(
						&keys::envoy::PoolNameKey::new(input.namespace_id, input.envoy_key.clone()),
						Serializable,
					)
					.await?
				{
					return Ok(None);
				}

				let pool_name_key =
					keys::envoy::PoolNameKey::new(input.namespace_id, input.envoy_key.clone());
				let version_key =
					keys::envoy::VersionKey::new(input.namespace_id, input.envoy_key.clone());
				let slots_key =
					keys::envoy::SlotsKey::new(input.namespace_id, input.envoy_key.clone());
				let create_ts_key =
					keys::envoy::CreateTsKey::new(input.namespace_id, input.envoy_key.clone());
				let connected_ts_key =
					keys::envoy::ConnectedTsKey::new(input.namespace_id, input.envoy_key.clone());
				let stop_ts_key =
					keys::envoy::StopTsKey::new(input.namespace_id, input.envoy_key.clone());
				let last_ping_ts_key =
					keys::envoy::LastPingTsKey::new(input.namespace_id, input.envoy_key.clone());
				let last_rtt_key =
					keys::envoy::LastRttKey::new(input.namespace_id, input.envoy_key.clone());
				let metadata_key =
					keys::envoy::MetadataKey::new(input.namespace_id, input.envoy_key.clone());
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
					envoy_key: input.envoy_key.clone(),
					namespace_id: input.namespace_id,
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
		})
		.custom_instrument(tracing::info_span!("envoy_get_tx"))
		.await
}
