use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::envoys::Envoy;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub pool_name: Option<String>,
	pub created_before: Option<i64>,
	pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub envoys: Vec<Envoy>,
}

#[operation]
pub async fn pegboard_envoy_list(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?;

	let envoys = ctx
		.udb()?
		.run(|tx| {
			let dc_name = dc_name.to_string();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				let mut results = Vec::new();

				if let Some(pool_name) = &input.pool_name {
					let envoy_subspace =
						keys::subspace().subspace(&keys::ns::ActiveEnvoyByNameKey::subspace(
							input.namespace_id,
							pool_name.clone(),
						));
					let (start, end) = envoy_subspace.range();

					let end = if let Some(created_before) = input.created_before {
						universaldb::utils::end_of_key_range(&tx.pack(
							&keys::ns::ActiveEnvoyByNameKey::subspace_with_create_ts(
								input.namespace_id,
								pool_name.clone(),
								created_before,
							),
						))
					} else {
						end
					};

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::Iterator,
							reverse: true,
							..(start, end).into()
						},
						// NOTE: Does not have to be serializable because we are listing, stale data does not matter
						Snapshot,
					);

					while let Some(entry) = stream.try_next().await? {
						let idx_key = tx.unpack::<keys::ns::ActiveEnvoyByNameKey>(entry.key())?;

						results.push(idx_key.envoy_key);

						if results.len() >= input.limit {
							break;
						}
					}
				} else {
					let envoy_subspace = keys::subspace()
						.subspace(&keys::ns::ActiveEnvoyKey::subspace(input.namespace_id));
					let (start, end) = envoy_subspace.range();

					let end = if let Some(created_before) = input.created_before {
						universaldb::utils::end_of_key_range(&tx.pack(
							&keys::ns::ActiveEnvoyKey::subspace_with_create_ts(
								input.namespace_id,
								created_before,
							),
						))
					} else {
						end
					};

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::Iterator,
							reverse: true,
							..(start, end).into()
						},
						// NOTE: Does not have to be serializable because we are listing, stale data does not matter
						Snapshot,
					);

					while let Some(entry) = stream.try_next().await? {
						let idx_key = tx.unpack::<keys::ns::ActiveEnvoyKey>(entry.key())?;

						results.push(idx_key.envoy_key);

						if results.len() >= input.limit {
							break;
						}
					}
				}

				futures_util::stream::iter(results)
					.map(|envoy_key| {
						let tx = tx.clone();
						let dc_name = dc_name.clone();

						async move {
							super::get::get_inner(&dc_name, &tx, input.namespace_id, &envoy_key)
								.await
						}
					})
					.buffered(512)
					.try_filter_map(|result| async move { Ok(result) })
					.try_collect::<Vec<_>>()
					.await
			}
		})
		.custom_instrument(tracing::info_span!("envoy_list_tx"))
		.await?;

	Ok(Output { envoys })
}
