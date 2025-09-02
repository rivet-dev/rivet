use anyhow::Result;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::runners::Runner;
use udb_util::{SNAPSHOT, TxnExt};
use universaldb::{self as udb, options::StreamingMode};

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub name: Option<String>,
	pub include_stopped: bool,
	pub created_before: Option<i64>,
	pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub runners: Vec<Runner>,
}

#[operation]
pub async fn pegboard_runner_list_for_ns(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?;

	let runners = ctx
		.udb()?
		.run(|tx, _mc| {
			let dc_name = dc_name.to_string();
			async move {
				let txs = tx.subspace(keys::subspace());
				let mut results = Vec::new();

				// TODO: Lots of duplicate code
				if let Some(name) = &input.name {
					if input.include_stopped {
						let runner_subspace =
							txs.subspace(&keys::ns::AllRunnerByNameKey::subspace(
								input.namespace_id,
								name.clone(),
							));
						let (start, end) = runner_subspace.range();

						let end = if let Some(created_before) = input.created_before {
							udb_util::end_of_key_range(&txs.pack(
								&keys::ns::AllRunnerByNameKey::subspace_with_create_ts(
									input.namespace_id,
									name.clone(),
									created_before,
								),
							))
						} else {
							end
						};

						let mut stream = txs.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::Iterator,
								reverse: true,
								..(start, end).into()
							},
							// NOTE: Does not have to be serializable because we are listing, stale data does not matter
							SNAPSHOT,
						);

						while let Some(entry) = stream.try_next().await? {
							let idx_key =
								txs.unpack::<keys::ns::AllRunnerByNameKey>(entry.key())?;

							results.push(idx_key.runner_id);

							if results.len() >= input.limit {
								break;
							}
						}
					} else {
						let runner_subspace =
							txs.subspace(&keys::ns::ActiveRunnerByNameKey::subspace(
								input.namespace_id,
								name.clone(),
							));
						let (start, end) = runner_subspace.range();

						let end = if let Some(created_before) = input.created_before {
							udb_util::end_of_key_range(&txs.pack(
								&keys::ns::ActiveRunnerByNameKey::subspace_with_create_ts(
									input.namespace_id,
									name.clone(),
									created_before,
								),
							))
						} else {
							end
						};

						let mut stream = txs.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::Iterator,
								reverse: true,
								..(start, end).into()
							},
							// NOTE: Does not have to be serializable because we are listing, stale data does not matter
							SNAPSHOT,
						);

						while let Some(entry) = stream.try_next().await? {
							let idx_key =
								txs.unpack::<keys::ns::ActiveRunnerByNameKey>(entry.key())?;

							results.push(idx_key.runner_id);

							if results.len() >= input.limit {
								break;
							}
						}
					}
				} else {
					if input.include_stopped {
						let runner_subspace =
							txs.subspace(&keys::ns::AllRunnerKey::subspace(input.namespace_id));
						let (start, end) = runner_subspace.range();

						let end = if let Some(created_before) = input.created_before {
							udb_util::end_of_key_range(&txs.pack(
								&keys::ns::AllRunnerKey::subspace_with_create_ts(
									input.namespace_id,
									created_before,
								),
							))
						} else {
							end
						};

						let mut stream = txs.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::Iterator,
								reverse: true,
								..(start, end).into()
							},
							// NOTE: Does not have to be serializable because we are listing, stale data does not matter
							SNAPSHOT,
						);

						while let Some(entry) = stream.try_next().await? {
							let idx_key = txs.unpack::<keys::ns::AllRunnerKey>(entry.key())?;

							results.push(idx_key.runner_id);

							if results.len() >= input.limit {
								break;
							}
						}
					} else {
						let runner_subspace =
							txs.subspace(&keys::ns::ActiveRunnerKey::subspace(input.namespace_id));
						let (start, end) = runner_subspace.range();

						let end = if let Some(created_before) = input.created_before {
							udb_util::end_of_key_range(&txs.pack(
								&keys::ns::ActiveRunnerKey::subspace_with_create_ts(
									input.namespace_id,
									created_before,
								),
							))
						} else {
							end
						};

						let mut stream = txs.get_ranges_keyvalues(
							udb::RangeOption {
								mode: StreamingMode::Iterator,
								reverse: true,
								..(start, end).into()
							},
							// NOTE: Does not have to be serializable because we are listing, stale data does not matter
							SNAPSHOT,
						);

						while let Some(entry) = stream.try_next().await? {
							let idx_key = txs.unpack::<keys::ns::ActiveRunnerKey>(entry.key())?;

							results.push(idx_key.runner_id);

							if results.len() >= input.limit {
								break;
							}
						}
					}
				}

				futures_util::stream::iter(results)
					.map(|runner_id| {
						let tx = tx.clone();
						let dc_name = dc_name.clone();

						async move { super::get::get_inner(&dc_name, &tx, runner_id).await }
					})
					.buffered(512)
					.try_filter_map(|result| async move { Ok(result) })
					.try_collect::<Vec<_>>()
					.await
			}
		})
		.await?;

	Ok(Output { runners })
}
