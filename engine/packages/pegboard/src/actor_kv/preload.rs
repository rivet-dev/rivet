use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_config::config::pegboard::Pegboard;
use rivet_envoy_protocol as ep;
use serde::Deserialize;
use universaldb::prelude::*;
use universaldb::tuple::Subspace;

use super::entry::EntryBuilder;
use crate::keys;

/// Request to preload a prefix range from the actor's KV store.
pub struct PreloadPrefixRequest {
	/// The raw key prefix bytes (e.g., [2] for connections, [8] for SQLite).
	pub prefix: ep::KvKey,
	/// Maximum bytes to preload for this prefix.
	pub max_bytes: u64,
	/// If true, return whatever fits even if truncated (for per-key lookup subsystems
	/// like SQLite VFS). If false, return nothing if the total data exceeds max_bytes
	/// (for list-based subsystems like connections and workflows).
	pub partial: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreloadConfig {
	#[serde(default)]
	keys: Vec<Vec<u8>>,
	#[serde(default)]
	prefixes: Vec<PreloadPrefixConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreloadPrefixConfig {
	prefix: Vec<u8>,
	max_bytes: u64,
	#[serde(default)]
	partial: bool,
}

/// Fetches all preload data for an actor in a single FDB snapshot transaction.
///
/// Reads exact get-keys and prefix ranges, reassembles chunked FDB values using
/// EntryBuilder, strips tuple encoding via KeyWrapper::unpack, and returns raw
/// byte key-value pairs ready for TypeScript consumption.
///
/// Prefix requests should be passed in descending priority order (highest priority
/// first). When the global byte cap is reached, lower-priority prefixes are
/// truncated first.
#[tracing::instrument(skip_all)]
pub(crate) async fn batch_preload(
	db: &universaldb::Database,
	actor_id: Id,
	get_keys: Vec<ep::KvKey>,
	prefix_requests: Vec<PreloadPrefixRequest>,
	max_total_bytes: u64,
) -> Result<ep::PreloadedKv> {
	let subspace = keys::actor_kv::subspace(actor_id);

	// Break prefix_requests into separate vectors so they can be cloned for the
	// FDB transaction closure (which may retry on conflicts).
	let prefix_keys: Vec<ep::KvKey> = prefix_requests.iter().map(|r| r.prefix.clone()).collect();
	let prefix_params: Vec<(u64, bool)> = prefix_requests
		.iter()
		.map(|r| (r.max_bytes, r.partial))
		.collect();

	db.run(|tx| {
		let subspace = subspace.clone();
		let get_keys = get_keys.clone();
		let prefix_keys = prefix_keys.clone();
		let prefix_params = prefix_params.clone();

		async move {
			let tx = tx.with_subspace(subspace.clone());
			let mut entries = Vec::new();
			let mut total_bytes: u64 = 0;

			// Build requested lists dynamically so they only contain keys/prefixes
			// that were actually scanned. Keys or prefixes skipped due to budget
			// exhaustion or disabled config must not appear, otherwise the actor
			// would mistake "not scanned" for "scanned and not found".
			let mut requested_get_keys: Vec<ep::KvKey> = Vec::new();
			let mut requested_prefixes: Vec<ep::KvKey> = Vec::new();

			// 1. Read exact get-keys. Each key maps to a single logical entry
			// (or nothing if the key doesn't exist in FDB).
			for key in &get_keys {
				if total_bytes >= max_total_bytes {
					tracing::debug!(
						skipped_keys = get_keys.len() - requested_get_keys.len(),
						"preload get-keys skipped due to global budget exhaustion"
					);
					break;
				}

				// Mark this key as scanned regardless of whether it exists in FDB.
				requested_get_keys.push(key.clone());

				let key_subspace = subspace.subspace(&keys::actor_kv::KeyWrapper(key.clone()));
				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: universaldb::options::StreamingMode::WantAll,
						..key_subspace.range().into()
					},
					Snapshot,
				);

				let mut builder: Option<EntryBuilder> = None;

				while let Some(fdb_kv) = stream.try_next().await? {
					if builder.is_none() {
						let parsed_key = tx
							.unpack::<keys::actor_kv::EntryBaseKey>(&fdb_kv.key())?
							.key;
						builder = Some(EntryBuilder::new(parsed_key));
					}

					let b = builder.as_mut().unwrap();

					if let Ok(chunk_key) =
						tx.unpack::<keys::actor_kv::EntryValueChunkKey>(&fdb_kv.key())
					{
						b.append_chunk(chunk_key.chunk, fdb_kv.value());
					} else if let Ok(metadata_key) =
						tx.unpack::<keys::actor_kv::EntryMetadataKey>(&fdb_kv.key())
					{
						let metadata = metadata_key.deserialize(fdb_kv.value())?;
						b.append_metadata(metadata);
					} else {
						bail!("unexpected sub key in preload get");
					}
				}

				if let Some(b) = builder {
					let (k, v, m) = b.build()?;
					let size = entry_size(&k, &v, &m);
					if total_bytes + size <= max_total_bytes {
						total_bytes += size;
						entries.push(ep::PreloadedKvEntry {
							key: k,
							value: v,
							metadata: m,
						});
					}
				}
			}

			// 2. Read prefix ranges in priority order. Each prefix is bounded by
			// its per-prefix max_bytes and the remaining global budget.
			for (i, prefix) in prefix_keys.iter().enumerate() {
				let (max_bytes, partial) = prefix_params[i];

				// Skip prefixes disabled by config (max_bytes == 0) or when
				// global budget is exhausted. Do not include in requested_prefixes
				// so the actor falls back to kvListPrefix.
				let remaining_budget = max_total_bytes.saturating_sub(total_bytes);
				let effective_limit = max_bytes.min(remaining_budget);

				if effective_limit == 0 {
					tracing::debug!(
						?prefix,
						max_bytes,
						remaining_budget,
						"preload prefix skipped, effective limit is 0"
					);
					continue;
				}

				let range = prefix_range(prefix, &subspace);
				let mut stream = tx.get_ranges_keyvalues(
					universaldb::RangeOption {
						mode: universaldb::options::StreamingMode::Iterator,
						..range.into()
					},
					Snapshot,
				);

				let mut prefix_entries: Vec<ep::PreloadedKvEntry> = Vec::new();
				let mut prefix_bytes: u64 = 0;
				let mut current_entry: Option<EntryBuilder> = None;
				let mut exceeded = false;

				while let Some(fdb_kv) = stream.try_next().await? {
					let key = tx
						.unpack::<keys::actor_kv::EntryBaseKey>(&fdb_kv.key())?
						.key;

					let curr = if let Some(inner) = &mut current_entry {
						if inner.key != key {
							// Finalize the previous entry.
							let prev = std::mem::replace(inner, EntryBuilder::new(key));
							let (k, v, m) = prev.build()?;
							let size = entry_size(&k, &v, &m);

							if prefix_bytes + size > effective_limit {
								exceeded = true;
								break;
							}

							prefix_bytes += size;
							prefix_entries.push(ep::PreloadedKvEntry {
								key: k,
								value: v,
								metadata: m,
							});
						}

						inner
					} else {
						current_entry = Some(EntryBuilder::new(key));
						current_entry.as_mut().expect("just set")
					};

					if let Ok(chunk_key) =
						tx.unpack::<keys::actor_kv::EntryValueChunkKey>(&fdb_kv.key())
					{
						curr.append_chunk(chunk_key.chunk, fdb_kv.value());
					} else if let Ok(metadata_key) =
						tx.unpack::<keys::actor_kv::EntryMetadataKey>(&fdb_kv.key())
					{
						let metadata = metadata_key.deserialize(fdb_kv.value())?;
						curr.append_metadata(metadata);
					} else {
						bail!("unexpected sub key in preload prefix scan");
					}
				}

				// Finalize the last entry if the stream ended without exceeding.
				if !exceeded {
					if let Some(b) = current_entry {
						let (k, v, m) = b.build()?;
						let size = entry_size(&k, &v, &m);
						if prefix_bytes + size > effective_limit {
							exceeded = true;
						} else {
							prefix_bytes += size;
							prefix_entries.push(ep::PreloadedKvEntry {
								key: k,
								value: v,
								metadata: m,
							});
						}
					}
				}

				// For non-partial prefixes, discard all entries if the data exceeded
				// the limit. The subsystem will fall back to a full kvListPrefix.
				// Do not include in requested_prefixes so the actor knows to fall back.
				if exceeded && !partial {
					tracing::debug!(
						?prefix,
						prefix_bytes,
						effective_limit,
						"preload prefix truncated (partial: false), discarding entries"
					);
					continue;
				}

				if exceeded {
					tracing::debug!(
						?prefix,
						prefix_bytes,
						effective_limit,
						"preload prefix truncated (partial: true), keeping partial data"
					);
				}

				requested_prefixes.push(prefix.clone());
				total_bytes += prefix_bytes;
				entries.extend(prefix_entries);
			}

			Ok(ep::PreloadedKv {
				entries,
				requested_get_keys,
				requested_prefixes,
			})
		}
	})
	.custom_instrument(tracing::info_span!("kv_batch_preload_tx"))
	.await
	.map_err(Into::<anyhow::Error>::into)
}

/// Fetches preloaded KV data for an actor using engine config and actor name
/// metadata. Returns `None` if preloading is disabled or missing from actor
/// metadata. Fails if the FDB transaction fails (no silent fallback).
#[tracing::instrument(skip_all)]
pub async fn fetch_preloaded_kv(
	db: &universaldb::Database,
	config: &Pegboard,
	actor_id: Id,
	namespace_id: Id,
	actor_name: &str,
) -> Result<Option<ep::PreloadedKv>> {
	// Read actor name metadata from FDB.
	let metadata = db
		.run(|tx| {
			let tx = tx.with_subspace(keys::subspace());
			let name_key = keys::ns::ActorNameKey::new(namespace_id, actor_name.to_string());
			async move { tx.read_opt(&name_key, Snapshot).await }
		})
		.instrument(tracing::info_span!("read_actor_metadata_tx"))
		.await?;

	let metadata_map = metadata
		.map(|d: rivet_data::converted::ActorNameKeyData| d.metadata)
		.unwrap_or_default();

	let Some(preload_config) = metadata_map
		.get("preload")
		.and_then(|value| serde_json::from_value::<PreloadConfig>(value.clone()).ok())
	else {
		return Ok(None);
	};

	let preload_max_total_bytes = config.preload_max_total_bytes();
	if preload_max_total_bytes == 0 {
		return Ok(None);
	};

	let prefix_requests = preload_config
		.prefixes
		.into_iter()
		.map(|prefix| PreloadPrefixRequest {
			prefix: prefix.prefix,
			max_bytes: prefix.max_bytes,
			partial: prefix.partial,
		})
		.collect();

	let preloaded = batch_preload(
		db,
		actor_id,
		preload_config.keys,
		prefix_requests,
		preload_max_total_bytes,
	)
	.await?;

	Ok(Some(preloaded))
}

/// Computes the serialized size of a preloaded KV entry, including key, value,
/// and metadata (version bytes + i64 timestamp).
fn entry_size(key: &ep::KvKey, value: &ep::KvValue, metadata: &ep::KvMetadata) -> u64 {
	(key.len() + value.len() + metadata.version.len() + std::mem::size_of::<i64>()) as u64
}

/// Computes the FDB key range for a prefix scan within the actor KV subspace.
fn prefix_range(prefix: &ep::KvKey, subspace: &Subspace) -> (Vec<u8>, Vec<u8>) {
	let mut start = subspace.pack(&keys::actor_kv::ListKeyWrapper(prefix.clone()));
	// Remove the trailing 0 byte that tuple encoding adds to bytes.
	if let Some(&0) = start.last() {
		start.pop();
	}
	let mut end = start.clone();
	end.push(0xFF);
	(start, end)
}
