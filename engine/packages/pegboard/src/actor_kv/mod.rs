use anyhow::Result;
use entry::EntryBuilder;
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_envoy_protocol as ep;
use std::sync::{Arc, Mutex};
use universaldb::prelude::*;
use universaldb::tuple::Subspace;
use utils::{validate_entries_with_details, validate_keys, validate_range};

use crate::keys;

mod entry;
mod metrics;
pub mod preload;
mod sqlite_telemetry;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Keep the KV validation limits below in sync with
// rivetkit-typescript/packages/rivetkit/src/drivers/file-system/kv-limits.ts.
pub const MAX_KEY_SIZE: usize = 2 * 1024;
pub const MAX_VALUE_SIZE: usize = 128 * 1024;
pub const MAX_KEYS: usize = 128;
pub const MAX_PUT_PAYLOAD_SIZE: usize = 976 * 1024;
const MAX_STORAGE_SIZE: usize = 10 * 1024 * 1024 * 1024; // 10 GiB
const VALUE_CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

// Namespace and name are used for metrics
pub struct Recipient {
	pub actor_id: Id,
	pub namespace_id: Id,
	pub name: String,
}

/// Returns estimated size of the given actor kv subspace.
#[tracing::instrument(skip_all)]
pub async fn estimate_kv_size(tx: &universaldb::Transaction, actor_id: Id) -> Result<i64> {
	let subspace = &keys::actor_kv::subspace(actor_id);
	let (start, end) = subspace.range();

	tx.get_estimated_range_size_bytes(&start, &end).await
}

/// Gets keys from the KV store.
#[tracing::instrument(skip_all)]
pub async fn get(
	db: &universaldb::Database,
	recipient: &Recipient,
	keys: Vec<ep::KvKey>,
) -> Result<(Vec<ep::KvKey>, Vec<ep::KvValue>, Vec<ep::KvMetadata>)> {
	let start = std::time::Instant::now();
	let sqlite_summary = sqlite_telemetry::summarize_get(&keys);
	metrics::ACTOR_KV_KEYS_PER_OP
		.with_label_values(&["get"])
		.observe(keys.len() as f64);
	validate_keys(&keys)?;

	let result = db
		.run(|tx| {
			let keys = keys.clone();
			async move {
				let tx = tx.with_subspace(keys::actor_kv::subspace(recipient.actor_id));

				let mut stream = futures_util::stream::iter(keys)
					.map(|key| {
						let key_subspace = keys::actor_kv::subspace(recipient.actor_id)
							.subspace(&keys::actor_kv::KeyWrapper(key));

						// Get all sub keys in the key subspace
						tx.get_ranges_keyvalues(
							universaldb::RangeOption {
								mode: universaldb::options::StreamingMode::WantAll,
								..key_subspace.range().into()
							},
							Serializable,
						)
					})
					.flatten();

				let mut keys = Vec::new();
				let mut values = Vec::new();
				let mut metadata = Vec::new();
				let mut total_size = 0;
				let mut current_entry: Option<EntryBuilder> = None;

				loop {
					let Some(entry) = stream.try_next().await? else {
						break;
					};

					total_size += entry.key().len() + entry.value().len();

					let key = tx.unpack::<keys::actor_kv::EntryBaseKey>(&entry.key())?.key;

					let current_entry = if let Some(inner) = &mut current_entry {
						if inner.key != key {
							let (key, value, meta) =
								std::mem::replace(inner, EntryBuilder::new(key)).build()?;

							keys.push(key);
							values.push(value);
							metadata.push(meta);
						}

						inner
					} else {
						current_entry = Some(EntryBuilder::new(key));

						current_entry.as_mut().expect("must be set")
					};

					if let Ok(chunk_key) =
						tx.unpack::<keys::actor_kv::EntryValueChunkKey>(&entry.key())
					{
						current_entry.append_chunk(chunk_key.chunk, entry.value());
					} else if let Ok(metadata_key) =
						tx.unpack::<keys::actor_kv::EntryMetadataKey>(&entry.key())
					{
						let value = metadata_key.deserialize(entry.value())?;

						current_entry.append_metadata(value);
					} else {
						bail!("unexpected sub key");
					}
				}

				if let Some(inner) = current_entry {
					let (key, value, meta) = inner.build()?;

					keys.push(key);
					values.push(value);
					metadata.push(meta);
				}

				// Total read bytes (rounded up to nearest chunk)
				let total_size_chunked = (total_size as u64)
					.div_ceil(util::metric::KV_BILLABLE_CHUNK)
					* util::metric::KV_BILLABLE_CHUNK;
				namespace::keys::metric::inc(
					&tx.with_subspace(namespace::keys::subspace()),
					recipient.namespace_id,
					namespace::keys::metric::Metric::KvRead(recipient.name.clone()),
					total_size_chunked.try_into().unwrap_or_default(),
				);

				Ok((keys, values, metadata))
			}
		})
		.custom_instrument(tracing::info_span!("kv_get_tx"))
		.await
		.map_err(Into::<anyhow::Error>::into);
	metrics::ACTOR_KV_OPERATION_DURATION
		.with_label_values(&["get"])
		.observe(start.elapsed().as_secs_f64());
	if let Some(summary) = sqlite_summary {
		let response_bytes = result
			.as_ref()
			.map(|(keys, values, _)| sqlite_telemetry::summarize_response(keys, values))
			.unwrap_or_default();
		sqlite_telemetry::record_response_bytes(response_bytes);
		sqlite_telemetry::record_operation(
			sqlite_telemetry::OperationKind::Read,
			summary,
			start.elapsed(),
		);
	}
	result
}

/// Gets keys from the KV store.
#[tracing::instrument(skip_all)]
pub async fn list(
	db: &universaldb::Database,
	recipient: &Recipient,
	query: ep::KvListQuery,
	reverse: bool,
	limit: Option<usize>,
) -> Result<(Vec<ep::KvKey>, Vec<ep::KvValue>, Vec<ep::KvMetadata>)> {
	utils::validate_list_query(&query)?;

	let limit = limit.unwrap_or(16384);
	let subspace = keys::actor_kv::subspace(recipient.actor_id);
	let list_range = list_query_range(query, &subspace);

	db.run(|tx| {
		let list_range = list_range.clone();
		let subspace = subspace.clone();

		async move {
			let tx = tx.with_subspace(subspace);

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: universaldb::options::StreamingMode::Iterator,
					reverse,
					..list_range.into()
				},
				Serializable,
			);

			let mut keys = Vec::new();
			let mut values = Vec::new();
			let mut metadata = Vec::new();
			let mut total_size = 0;
			let mut current_entry: Option<EntryBuilder> = None;

			loop {
				let Some(entry) = stream.try_next().await? else {
					break;
				};

				total_size += entry.key().len() + entry.value().len();

				let key = tx.unpack::<keys::actor_kv::EntryBaseKey>(&entry.key())?.key;

				let curr = if let Some(inner) = &mut current_entry {
					if inner.key != key {
						// Check limit before adding the key
						if keys.len() >= limit {
							current_entry = None;
							break;
						}

						let (key, value, meta) =
							std::mem::replace(inner, EntryBuilder::new(key)).build()?;

						keys.push(key);
						values.push(value);
						metadata.push(meta);
					}

					inner
				} else {
					current_entry = Some(EntryBuilder::new(key));

					current_entry.as_mut().expect("must be set")
				};

				if let Ok(chunk_key) = tx.unpack::<keys::actor_kv::EntryValueChunkKey>(&entry.key())
				{
					curr.append_chunk(chunk_key.chunk, entry.value());
				} else if let Ok(metadata_key) =
					tx.unpack::<keys::actor_kv::EntryMetadataKey>(&entry.key())
				{
					let value = metadata_key.deserialize(entry.value())?;

					curr.append_metadata(value);
				} else {
					bail!("unexpected sub key");
				}
			}

			// Only add the current entry if we haven't hit the limit yet
			if let Some(inner) = current_entry {
				if keys.len() < limit {
					let (key, value, meta) = inner.build()?;

					keys.push(key);
					values.push(value);
					metadata.push(meta);
				}
			}

			// Total read bytes (rounded up to nearest chunk)
			let total_size_chunked = (total_size as u64).div_ceil(util::metric::KV_BILLABLE_CHUNK)
				* util::metric::KV_BILLABLE_CHUNK;
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				recipient.namespace_id,
				namespace::keys::metric::Metric::KvRead(recipient.name.clone()),
				total_size_chunked.try_into().unwrap_or_default(),
			);

			Ok((keys, values, metadata))
		}
	})
	.custom_instrument(tracing::info_span!("kv_list_tx"))
	.await
	.map_err(Into::<anyhow::Error>::into)
}

/// Puts keys into the KV store.
#[tracing::instrument(skip_all)]
pub async fn put(
	db: &universaldb::Database,
	recipient: &Recipient,
	keys: Vec<ep::KvKey>,
	values: Vec<ep::KvValue>,
) -> Result<()> {
	let start = std::time::Instant::now();
	let sqlite_summary = sqlite_telemetry::summarize_put(&keys, &values);
	let sqlite_observation = Arc::new(Mutex::new(SqliteWriteObservation::default()));
	metrics::ACTOR_KV_KEYS_PER_OP
		.with_label_values(&["put"])
		.observe(keys.len() as f64);
	let keys = &keys;
	let values = &values;
	let sqlite_observation_clone = Arc::clone(&sqlite_observation);
	let result = db
		.run(|tx| {
			let sqlite_observation = Arc::clone(&sqlite_observation_clone);
			async move {
				let estimate_start = std::time::Instant::now();
				let total_size = estimate_kv_size(&tx, recipient.actor_id).await;
				observe_sqlite_write(
					|observation| {
						observation.estimate_kv_size_duration = estimate_start.elapsed();
						observation.estimate_kv_size_recorded = true;
					},
					&sqlite_observation,
				);
				let total_size = total_size? as usize;

				match validate_entries_with_details(&keys, &values, total_size) {
					Ok(()) => {
						observe_sqlite_write(
							|observation| {
								observation.validation_checked = true;
								observation.validation_result = None;
							},
							&sqlite_observation,
						);
					}
					Err(error) => {
						observe_sqlite_write(
							|observation| {
								observation.validation_checked = true;
								observation.validation_result = Some(error.kind());
							},
							&sqlite_observation,
						);
						return Err(error.into_anyhow());
					}
				}

				let subspace = &keys::actor_kv::subspace(recipient.actor_id);
				let tx = tx.with_subspace(subspace.clone());
				let now = util::timestamp::now();

				// TODO: Include metadata size?
				// Total written bytes (rounded up to nearest chunk)
				let total_size = keys.iter().fold(0, |s, key| s + key.len())
					+ values.iter().fold(0, |s, value| s + value.len());
				let total_size_chunked = (total_size as u64)
					.div_ceil(util::metric::KV_BILLABLE_CHUNK)
					* util::metric::KV_BILLABLE_CHUNK;
				namespace::keys::metric::inc(
					&tx.with_subspace(namespace::keys::subspace()),
					recipient.namespace_id,
					namespace::keys::metric::Metric::KvWrite(recipient.name.clone()),
					total_size_chunked.try_into().unwrap_or_default(),
				);

				let rewrite_start = std::time::Instant::now();
				let write_result = futures_util::stream::iter(0..keys.len())
					.map(|i| {
						let tx = tx.clone();
						async move {
							// TODO: Costly clone
							let key = keys::actor_kv::KeyWrapper(
								keys.get(i).context("index should exist")?.clone(),
							);
							let value = values.get(i).context("index should exist")?;
							// Clear previous key data before setting
							tx.clear_subspace_range(&subspace.subspace(&key));

							// Set metadata
							tx.write(
								&keys::actor_kv::EntryMetadataKey::new(key.clone()),
								ep::KvMetadata {
									version: VERSION.as_bytes().to_vec(),
									update_ts: now,
								},
							)?;

							// Set key data in chunks
							for start in (0..value.len()).step_by(VALUE_CHUNK_SIZE) {
								let idx = start / VALUE_CHUNK_SIZE;
								let end = (start + VALUE_CHUNK_SIZE).min(value.len());

								tx.set(
									&subspace.pack(&keys::actor_kv::EntryValueChunkKey::new(
										key.clone(),
										idx,
									)),
									&value.get(start..end).context("bad slice")?,
								);
							}

							Ok(())
						}
					})
					.buffer_unordered(32)
					.try_collect()
					.await;
				observe_sqlite_write(
					|observation| {
						observation.clear_and_rewrite_duration = rewrite_start.elapsed();
						observation.clear_and_rewrite_recorded = true;
					},
					&sqlite_observation,
				);
				write_result
			}
		})
		.custom_instrument(tracing::info_span!("kv_put_tx"))
		.await
		.map_err(Into::into);
	metrics::ACTOR_KV_OPERATION_DURATION
		.with_label_values(&["put"])
		.observe(start.elapsed().as_secs_f64());
	if let Some(summary) = sqlite_summary {
		let observation = sqlite_observation
			.lock()
			.ok()
			.map(|guard| *guard)
			.unwrap_or_default();
		if observation.validation_checked {
			sqlite_telemetry::record_validation(observation.validation_result);
		}
		if observation.estimate_kv_size_recorded {
			sqlite_telemetry::record_phase_duration(
				sqlite_telemetry::PhaseKind::EstimateKvSize,
				observation.estimate_kv_size_duration,
			);
		}
		if observation.clear_and_rewrite_recorded {
			sqlite_telemetry::record_phase_duration(
				sqlite_telemetry::PhaseKind::ClearAndRewrite,
				observation.clear_and_rewrite_duration,
			);
			sqlite_telemetry::record_clear_subspace(summary.entry_count());
		}
		sqlite_telemetry::record_operation(
			sqlite_telemetry::OperationKind::Write,
			summary,
			start.elapsed(),
		);
	}
	result
}

/// Deletes keys from the KV store. Cannot be undone.
#[tracing::instrument(skip_all)]
pub async fn delete(
	db: &universaldb::Database,
	recipient: &Recipient,
	keys: Vec<ep::KvKey>,
) -> Result<()> {
	let start = std::time::Instant::now();
	metrics::ACTOR_KV_KEYS_PER_OP
		.with_label_values(&["delete"])
		.observe(keys.len() as f64);
	validate_keys(&keys)?;

	let keys = &keys;
	let result = db
		.run(|tx| {
			async move {
				// Total written bytes (rounded up to nearest chunk)
				let total_size = keys.iter().fold(0, |s, key| s + key.len());
				let total_size_chunked = (total_size as u64)
					.div_ceil(util::metric::KV_BILLABLE_CHUNK)
					* util::metric::KV_BILLABLE_CHUNK;
				namespace::keys::metric::inc(
					&tx.with_subspace(namespace::keys::subspace()),
					recipient.namespace_id,
					namespace::keys::metric::Metric::KvWrite(recipient.name.clone()),
					total_size_chunked.try_into().unwrap_or_default(),
				);

				for key in keys {
					// TODO: Costly clone
					let key_subspace = keys::actor_kv::subspace(recipient.actor_id)
						.subspace(&keys::actor_kv::KeyWrapper(key.clone()));

					tx.clear_subspace_range(&key_subspace);
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("kv_delete_tx"))
		.await
		.map_err(Into::into);
	metrics::ACTOR_KV_OPERATION_DURATION
		.with_label_values(&["delete"])
		.observe(start.elapsed().as_secs_f64());
	result
}

/// Deletes all keys in the half-open range [start, end). Cannot be undone.
#[tracing::instrument(skip_all)]
pub async fn delete_range(
	db: &universaldb::Database,
	recipient: &Recipient,
	start: ep::KvKey,
	end: ep::KvKey,
) -> Result<()> {
	let timer = std::time::Instant::now();
	let sqlite_summary = sqlite_telemetry::summarize_delete_range(&start, &end);
	validate_range(&start, &end)?;
	if start >= end {
		metrics::ACTOR_KV_OPERATION_DURATION
			.with_label_values(&["delete_range"])
			.observe(timer.elapsed().as_secs_f64());
		if let Some(summary) = sqlite_summary {
			sqlite_telemetry::record_operation(
				sqlite_telemetry::OperationKind::Truncate,
				summary,
				timer.elapsed(),
			);
		}
		return Ok(());
	}

	let result = db
		.run(|tx| {
			let start = start.clone();
			let end = end.clone();
			async move {
				// Total written bytes (rounded up to nearest chunk)
				let total_size = start.len() + end.len();
				let total_size_chunked = (total_size as u64)
					.div_ceil(util::metric::KV_BILLABLE_CHUNK)
					* util::metric::KV_BILLABLE_CHUNK;
				namespace::keys::metric::inc(
					&tx.with_subspace(namespace::keys::subspace()),
					recipient.namespace_id,
					namespace::keys::metric::Metric::KvWrite(recipient.name.clone()),
					total_size_chunked.try_into().unwrap_or_default(),
				);

				let subspace = keys::actor_kv::subspace(recipient.actor_id);
				let begin = subspace
					.subspace(&keys::actor_kv::KeyWrapper(start))
					.range()
					.0;
				let end = subspace
					.subspace(&keys::actor_kv::KeyWrapper(end))
					.range()
					.0;
				tx.clear_range(&begin, &end);

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("kv_delete_range_tx"))
		.await
		.map_err(Into::into);
	metrics::ACTOR_KV_OPERATION_DURATION
		.with_label_values(&["delete_range"])
		.observe(timer.elapsed().as_secs_f64());
	if let Some(summary) = sqlite_summary {
		sqlite_telemetry::record_operation(
			sqlite_telemetry::OperationKind::Truncate,
			summary,
			timer.elapsed(),
		);
	}
	result
}

#[derive(Clone, Copy, Debug, Default)]
struct SqliteWriteObservation {
	estimate_kv_size_duration: std::time::Duration,
	estimate_kv_size_recorded: bool,
	clear_and_rewrite_duration: std::time::Duration,
	clear_and_rewrite_recorded: bool,
	validation_checked: bool,
	validation_result: Option<utils::EntryValidationErrorKind>,
}

fn observe_sqlite_write(
	update: impl FnOnce(&mut SqliteWriteObservation),
	observation: &Arc<Mutex<SqliteWriteObservation>>,
) {
	if let Ok(mut guard) = observation.lock() {
		update(&mut guard);
	}
}

/// Deletes all keys from the KV store. Cannot be undone.
#[tracing::instrument(skip_all)]
pub async fn delete_all(db: &universaldb::Database, recipient: &Recipient) -> Result<()> {
	db.run(|tx| async move {
		tx.clear_subspace_range(&keys::actor_kv::subspace(recipient.actor_id));

		// Total written bytes (rounded up to nearest chunk)
		namespace::keys::metric::inc(
			&tx.with_subspace(namespace::keys::subspace()),
			recipient.namespace_id,
			namespace::keys::metric::Metric::KvWrite(recipient.name.clone()),
			util::metric::KV_BILLABLE_CHUNK
				.try_into()
				.unwrap_or_default(),
		);

		Ok(())
	})
	.custom_instrument(tracing::info_span!("kv_delete_all_tx"))
	.await
	.map_err(Into::into)
}

fn list_query_range(query: ep::KvListQuery, subspace: &Subspace) -> (Vec<u8>, Vec<u8>) {
	match query {
		ep::KvListQuery::KvListAllQuery => subspace.range(),
		ep::KvListQuery::KvListRangeQuery(range) => (
			subspace
				.subspace(&keys::actor_kv::ListKeyWrapper(range.start))
				.range()
				.0,
			if range.exclusive {
				subspace
					.subspace(&keys::actor_kv::KeyWrapper(range.end))
					.range()
					.0
			} else {
				subspace
					.subspace(&keys::actor_kv::KeyWrapper(range.end))
					.range()
					.1
			},
		),
		ep::KvListQuery::KvListPrefixQuery(prefix) => {
			// For prefix queries, we need to create a range that matches all keys
			// that start with the given prefix bytes. The tuple encoding adds a
			// terminating 0 byte to strings, which would make the range too narrow.
			//
			// Instead, we construct the range manually:
			// - Start: the prefix bytes within the subspace
			// - End: the prefix bytes + 0xFF (next possible byte)

			let mut start = subspace.pack(&keys::actor_kv::ListKeyWrapper(prefix.key.clone()));
			// Remove the trailing 0 byte that tuple encoding adds to strings
			if let Some(&0) = start.last() {
				start.pop();
			}

			let mut end = start.clone();
			end.push(0xFF);

			(start, end)
		}
	}
}
