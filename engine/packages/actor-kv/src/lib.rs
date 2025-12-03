use std::result::Result::Ok;

use anyhow::*;
use entry::{EntryBaseKey, EntryBuilder, EntryMetadataKey, EntryValueChunkKey};
use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use key::{KeyWrapper, ListKeyWrapper};
use rivet_runner_protocol::mk2 as rp;
use universaldb::prelude::*;
use universaldb::tuple::Subspace;
use utils::{validate_entries, validate_keys};

mod entry;
mod key;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_KEY_SIZE: usize = 2 * 1024;
const MAX_VALUE_SIZE: usize = 128 * 1024;
const MAX_KEYS: usize = 128;
const MAX_PUT_PAYLOAD_SIZE: usize = 976 * 1024;
const MAX_STORAGE_SIZE: usize = 1024 * 1024 * 1024; // 1 GiB
const VALUE_CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

fn subspace(actor_id: Id) -> universaldb::utils::Subspace {
	pegboard::keys::actor_kv_subspace().subspace(&actor_id)
}

/// Returns estimated size of the given subspace.
#[tracing::instrument(skip_all)]
pub async fn get_subspace_size(db: &universaldb::Database, subspace: &Subspace) -> Result<i64> {
	let (start, end) = subspace.range();

	// This txn does not have to be committed because we are not modifying any data
	let tx = db.create_trx()?;
	tx.get_estimated_range_size_bytes(&start, &end)
		.await
		.map_err(Into::into)
}

/// Gets keys from the KV store.
#[tracing::instrument(skip_all)]
pub async fn get(
	db: &universaldb::Database,
	actor_id: Id,
	keys: Vec<rp::KvKey>,
) -> Result<(Vec<rp::KvKey>, Vec<rp::KvValue>, Vec<rp::KvMetadata>)> {
	validate_keys(&keys)?;

	db.run(|tx| {
		let keys = keys.clone();
		async move {
			let tx = tx.with_subspace(subspace(actor_id));

			let size_estimate = keys.len().min(1024);

			let mut stream = futures_util::stream::iter(keys)
				.map(|key| {
					let key_subspace = subspace(actor_id).subspace(&KeyWrapper(key));

					// Get all sub keys in the key subspace
					tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: universaldb::options::StreamingMode::WantAll,
							..key_subspace.range().into()
						},
						Serializable,
					)
				})
				// Should remain in order
				// .buffered(32)
				.flatten();

			let mut keys = Vec::with_capacity(size_estimate);
			let mut values = Vec::with_capacity(size_estimate);
			let mut metadata = Vec::with_capacity(size_estimate);
			let mut current_entry: Option<EntryBuilder> = None;

			loop {
				let Some(entry) = stream.try_next().await? else {
					break;
				};

				let key = tx.unpack::<EntryBaseKey>(&entry.key())?.key;

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

				if let Ok(chunk_key) = tx.unpack::<EntryValueChunkKey>(&entry.key()) {
					current_entry.append_chunk(chunk_key.chunk, entry.value());
				} else if let Ok(metadata_key) = tx.unpack::<EntryMetadataKey>(&entry.key()) {
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

			Ok((keys, values, metadata))
		}
	})
	.custom_instrument(tracing::info_span!("kv_get_tx"))
	.await
	.map_err(Into::<anyhow::Error>::into)
}

/// Gets keys from the KV store.
#[tracing::instrument(skip_all)]
pub async fn list(
	db: &universaldb::Database,
	actor_id: Id,
	query: rp::KvListQuery,
	reverse: bool,
	limit: Option<usize>,
) -> Result<(Vec<rp::KvKey>, Vec<rp::KvValue>, Vec<rp::KvMetadata>)> {
	utils::validate_list_query(&query)?;

	let limit = limit.unwrap_or(16384);
	let subspace = subspace(actor_id);
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
			let mut current_entry: Option<EntryBuilder> = None;

			loop {
				let Some(entry) = stream.try_next().await? else {
					break;
				};

				let key = tx.unpack::<EntryBaseKey>(&entry.key())?.key;

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

				if let Ok(chunk_key) = tx.unpack::<EntryValueChunkKey>(&entry.key()) {
					curr.append_chunk(chunk_key.chunk, entry.value());
				} else if let Ok(metadata_key) = tx.unpack::<EntryMetadataKey>(&entry.key()) {
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
	actor_id: Id,
	keys: Vec<rp::KvKey>,
	values: Vec<rp::KvValue>,
) -> Result<()> {
	let subspace = subspace(actor_id);
	let total_size = get_subspace_size(&db, &subspace).await? as usize;

	validate_entries(&keys, &values, total_size)?;

	db.run(|tx| {
		// TODO: Costly clone
		let keys = keys.clone();
		let values = values.clone();
		let subspace = subspace.clone();

		async move {
			let tx = tx.with_subspace(subspace.clone());

			futures_util::stream::iter(keys.into_iter().zip(values.into_iter()))
				.map(|(key, value)| {
					let tx = tx.clone();
					let key = KeyWrapper(key.clone());
					let subspace = subspace.clone();

					async move {
						// Clear previous key data before setting
						tx.clear_subspace_range(&subspace.subspace(&key));

						// Set metadata
						tx.write(
							&EntryMetadataKey::new(key.clone()),
							rp::KvMetadata {
								version: VERSION.as_bytes().to_vec(),
								create_ts: utils::now(),
							},
						)?;

						// Set key data in chunks
						for start in (0..value.len()).step_by(VALUE_CHUNK_SIZE) {
							let idx = start / VALUE_CHUNK_SIZE;
							let end = (start + VALUE_CHUNK_SIZE).min(value.len());

							tx.set(
								&subspace.pack(&EntryValueChunkKey::new(key.clone(), idx)),
								&value.get(start..end).context("bad slice")?,
							);
						}

						Ok(())
					}
				})
				.buffer_unordered(32)
				.try_collect()
				.await
		}
	})
	.custom_instrument(tracing::info_span!("kv_put_tx"))
	.await
	.map_err(Into::into)
}

/// Deletes keys from the KV store. Cannot be undone.
#[tracing::instrument(skip_all)]
pub async fn delete(db: &universaldb::Database, actor_id: Id, keys: Vec<rp::KvKey>) -> Result<()> {
	validate_keys(&keys)?;

	db.run(|tx| {
		let keys = keys.clone();
		async move {
			for key in keys {
				let key_subspace = subspace(actor_id).subspace(&KeyWrapper(key));

				tx.clear_subspace_range(&key_subspace);
			}

			Ok(())
		}
	})
	.custom_instrument(tracing::info_span!("kv_delete_tx"))
	.await
	.map_err(Into::into)
}

/// Deletes all keys from the KV store. Cannot be undone.
#[tracing::instrument(skip_all)]
pub async fn delete_all(db: &universaldb::Database, actor_id: Id) -> Result<()> {
	db.run(|tx| async move {
		tx.clear_subspace_range(&subspace(actor_id));
		Ok(())
	})
	.custom_instrument(tracing::info_span!("kv_delete_all_tx"))
	.await
	.map_err(Into::into)
}

fn list_query_range(query: rp::KvListQuery, subspace: &Subspace) -> (Vec<u8>, Vec<u8>) {
	match query {
		rp::KvListQuery::KvListAllQuery => subspace.range(),
		rp::KvListQuery::KvListRangeQuery(range) => (
			subspace.subspace(&ListKeyWrapper(range.start)).range().0,
			if range.exclusive {
				subspace.subspace(&KeyWrapper(range.end)).range().0
			} else {
				subspace.subspace(&KeyWrapper(range.end)).range().1
			},
		),
		rp::KvListQuery::KvListPrefixQuery(prefix) => {
			// For prefix queries, we need to create a range that matches all keys
			// that start with the given prefix bytes. The tuple encoding adds a
			// terminating 0 byte to strings, which would make the range too narrow.
			//
			// Instead, we construct the range manually:
			// - Start: the prefix bytes within the subspace
			// - End: the prefix bytes + 0xFF (next possible byte)

			let mut start = subspace.pack(&ListKeyWrapper(prefix.key.clone()));
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
