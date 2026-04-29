//! Page read paths for sqlite-storage.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use scc::hash_map::Entry;

use crate::engine::{DecodedLtxCacheEntry, SqliteEngine};
use crate::error::SqliteStorageError;
use crate::keys::{
	decode_delta_chunk_txid, delta_chunk_prefix, delta_prefix, meta_key, pidx_delta_prefix,
	shard_key,
};
use crate::ltx::{DecodedLtx, decode_ltx_v3};
use crate::page_index::DeltaPageIndex;
use crate::types::{
	DBHead, FetchedPage, GetPagesResult, SQLITE_MAX_DELTA_BYTES, SqliteMeta, decode_db_head,
};
use crate::udb;

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();
pub const SQLITE_GET_PAGE_RANGE_MAX_PAGES: u32 = 256;
pub const SQLITE_GET_PAGE_RANGE_MAX_BYTES: u64 = 1024 * 1024;

impl SqliteEngine {
	pub async fn get_pages(
		&self,
		actor_id: &str,
		generation: u64,
		pgnos: Vec<u32>,
	) -> Result<GetPagesResult> {
		self.read_pages(actor_id, generation, PageReadRequest::Pages(pgnos))
			.await
	}

	pub async fn get_page_range(
		&self,
		actor_id: &str,
		generation: u64,
		start_pgno: u32,
		max_pages: u32,
		max_bytes: u64,
	) -> Result<GetPagesResult> {
		self.read_pages(
			actor_id,
			generation,
			PageReadRequest::Range {
				start_pgno,
				max_pages,
				max_bytes,
			},
		)
		.await
	}

	async fn read_pages(
		&self,
		actor_id: &str,
		generation: u64,
		request: PageReadRequest,
	) -> Result<GetPagesResult> {
		let start = Instant::now();
		request.validate()?;
		let operation = request.operation();
		self.ensure_open(actor_id, generation, operation).await?;

		let pidx_lookup_pgnos = request.pidx_lookup_pgnos();
		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let cached_pidx = match self.page_indices.get_async(&actor_id).await {
			Some(entry) => Some(
				pidx_lookup_pgnos
					.iter()
					.map(|pgno| (*pgno, entry.get().get(*pgno)))
					.collect::<BTreeMap<_, _>>(),
			),
			None => None,
		};
		let tx_result = udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			let cached_pidx = cached_pidx.clone();
			let request = request.clone();
			async move {
				let meta_key = meta_key(&actor_id);
				let head =
					if let Some(meta_bytes) = udb::tx_get_value(&tx, &subspace, &meta_key).await? {
						decode_db_head(&meta_bytes)?
					} else {
						ensure!(
							generation == 1,
							SqliteStorageError::MetaMissing { operation }
						);
						return Err(SqliteStorageError::MetaMissing { operation }.into());
					};
				ensure!(
					head.generation == generation,
					SqliteStorageError::FenceMismatch {
						reason: format!(
							"sqlite generation fence mismatch: expected {}, got {}",
							generation, head.generation
						),
					}
				);

				let pgnos = request.pgnos_for_head(&head);
				let pgnos_in_range = pgnos
					.iter()
					.copied()
					.filter(|pgno| *pgno <= head.db_size_pages)
					.collect::<Vec<_>>();
				if pgnos_in_range.is_empty() {
					return Ok(GetPagesTxResult {
						head,
						pgnos,
						loaded_pidx_rows: None,
						page_sources: BTreeMap::new(),
						source_blobs: BTreeMap::new(),
						pidx_hits: 0,
						pidx_misses: 0,
						stale_pidx_pgnos: BTreeSet::new(),
					});
				}

				let mut pidx_by_pgno = BTreeMap::new();
				let mut loaded_pidx_rows = None;
				if let Some(cached_pidx) = cached_pidx.as_ref() {
					for (pgno, txid) in cached_pidx {
						if let Some(txid) = txid {
							pidx_by_pgno.insert(*pgno, *txid);
						}
					}
				} else {
					let rows =
						udb::tx_scan_prefix_values(&tx, &subspace, &pidx_delta_prefix(&actor_id))
							.await?;
					let decoded_rows = rows
						.into_iter()
						.map(|(key, value)| {
							Ok((
								decode_pidx_pgno(&actor_id, &key)?,
								decode_pidx_txid(&value)?,
							))
						})
						.collect::<Result<Vec<_>>>()?;
					for (pgno, txid) in &decoded_rows {
						pidx_by_pgno.insert(*pgno, *txid);
					}
					loaded_pidx_rows = Some(decoded_rows);
				}

				let mut page_sources = BTreeMap::new();
				let mut source_blobs = BTreeMap::new();
				let mut missing_delta_keys = BTreeSet::new();
				let mut stale_pidx_pgnos = BTreeSet::new();
				let mut pidx_hits = 0usize;
				let mut pidx_misses = 0usize;

				for pgno in &pgnos_in_range {
					let preferred_delta_key = pidx_by_pgno.get(pgno).copied().map(|txid| {
						pidx_hits += 1;
						delta_chunk_prefix(&actor_id, txid)
					});
					if preferred_delta_key.is_none() {
						pidx_misses += 1;
					}

					let mut source_key = preferred_delta_key
						.clone()
						.unwrap_or_else(|| shard_key(&actor_id, *pgno / head.shard_size));
					if preferred_delta_key
						.as_ref()
						.is_some_and(|key| missing_delta_keys.contains(key))
					{
						source_key = shard_key(&actor_id, *pgno / head.shard_size);
						stale_pidx_pgnos.insert(*pgno);
					}

					if !source_blobs.contains_key(&source_key) {
						let mut blob = if source_key.starts_with(&delta_prefix(&actor_id)) {
							load_delta_blob_tx(&tx, &subspace, &source_key).await?
						} else {
							udb::tx_get_value(&tx, &subspace, &source_key).await?
						};
						if blob.is_none() {
							if let Some(delta_key) = preferred_delta_key.as_ref() {
								missing_delta_keys.insert(delta_key.clone());
								stale_pidx_pgnos.insert(*pgno);
								source_key = shard_key(&actor_id, *pgno / head.shard_size);
								blob = match source_blobs.get(&source_key).cloned() {
									Some(existing) => Some(existing),
									None => udb::tx_get_value(&tx, &subspace, &source_key).await?,
								};
							}
						}
						if let Some(blob) = blob {
							source_blobs.insert(source_key.clone(), blob);
						} else {
							continue;
						}
					}

					page_sources.insert(*pgno, source_key);
				}

				Ok(GetPagesTxResult {
					head,
					pgnos,
					loaded_pidx_rows,
					page_sources,
					source_blobs,
					pidx_hits,
					pidx_misses,
					stale_pidx_pgnos,
				})
			}
		})
		.await
		.map_err(|err| {
			if err
				.chain()
				.any(|cause| cause.to_string().contains("generation fence mismatch"))
			{
				self.metrics.inc_fence_mismatch_total();
			}
			err
		})?;
		let GetPagesTxResult {
			head,
			pgnos,
			loaded_pidx_rows,
			page_sources,
			source_blobs,
			pidx_hits,
			pidx_misses,
			stale_pidx_pgnos,
		} = tx_result;
		let requested_page_count = pgnos.len();
		let mut stale_pidx_pgnos = stale_pidx_pgnos;
		if let Some(loaded_pidx_rows) = loaded_pidx_rows {
			let loaded_index = DeltaPageIndex::new();
			for (pgno, txid) in loaded_pidx_rows {
				if !stale_pidx_pgnos.contains(&pgno) {
					loaded_index.insert(pgno, txid);
				}
			}
			match self.page_indices.entry_async(actor_id.clone()).await {
				Entry::Occupied(entry) => {
					for (pgno, txid) in loaded_index.range(0, u32::MAX) {
						entry.get().insert(pgno, txid);
					}
				}
				Entry::Vacant(entry) => {
					entry.insert_entry(loaded_index);
				}
			}
		}
		if page_sources.is_empty() && head.head_txid == 0 {
			self.metrics
				.observe_get_pages(requested_page_count, start.elapsed());
			let db_size_pages = head.db_size_pages;
			let page_size = head.page_size as usize;
			let meta = SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES));
			return Ok(GetPagesResult {
				pages: pgnos
					.into_iter()
					.map(|pgno| FetchedPage {
						pgno,
						bytes: if pgno <= db_size_pages {
							Some(vec![0; page_size])
						} else {
							None
						},
					})
					.collect(),
				meta,
			});
		}
		let mut decoded_blobs = BTreeMap::<Vec<u8>, Arc<DecodedLtx>>::new();
		let mut historical_delta_blobs = None;
		let mut pages = Vec::with_capacity(pgnos.len());

		for pgno in pgnos {
			if pgno > head.db_size_pages {
				pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let mut bytes = None;
			if let Some(source_key) = page_sources.get(&pgno) {
				let blob = source_blobs
					.get(source_key)
					.cloned()
					.with_context(|| format!("missing source blob for page {pgno}"))?;

				let decoded = self
					.decode_ltx_source_blob(
						&mut decoded_blobs,
						source_key,
						&blob,
						|| format!("decode source blob for page {pgno}"),
					)
					.await?;

				bytes = decoded
					.get_page(pgno)
					.map(ToOwned::to_owned);
				if bytes.is_none() {
					let shard_source_key = shard_key(&actor_id, pgno / head.shard_size);
					if source_key != &shard_source_key {
						stale_pidx_pgnos.insert(pgno);

						if let Some(shard_blob) = udb::get_value(
							&self.db,
							&self.subspace,
							self.op_counter.as_ref(),
							shard_source_key.clone(),
						)
						.await?
						{
							let decoded = self
								.decode_ltx_source_blob(
									&mut decoded_blobs,
									&shard_source_key,
									&shard_blob,
									|| format!("decode shard source blob for stale page {pgno}"),
								)
								.await?;
							bytes = decoded.get_page(pgno).map(ToOwned::to_owned);
						}
					}
				}
			}
			if bytes.is_none() {
				stale_pidx_pgnos.insert(pgno);
				if historical_delta_blobs.is_none() {
					historical_delta_blobs = Some(load_delta_history_blobs(self, &actor_id).await?);
				}
				bytes = recover_page_from_delta_history(
					&actor_id,
					pgno,
					&mut decoded_blobs,
					historical_delta_blobs
						.as_ref()
						.expect("historical delta blobs should load before recovery"),
				)?;
			}
			let bytes = bytes.unwrap_or_else(|| vec![0; head.page_size as usize]);

			pages.push(FetchedPage {
				pgno,
				bytes: Some(bytes),
			});
		}
		if !stale_pidx_pgnos.is_empty() {
			match self.page_indices.entry_async(actor_id.clone()).await {
				Entry::Occupied(entry) => {
					for pgno in stale_pidx_pgnos {
						entry.get().remove(pgno);
					}
				}
				Entry::Vacant(entry) => {
					drop(entry);
				}
			}
		}
		self.metrics.add_pidx_hits(pidx_hits);
		self.metrics.add_pidx_misses(pidx_misses);
		self.metrics
			.observe_get_pages(requested_page_count, start.elapsed());

		Ok(GetPagesResult {
			pages,
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
		})
	}

	async fn decode_ltx_source_blob(
		&self,
		decoded_blobs: &mut BTreeMap<Vec<u8>, Arc<DecodedLtx>>,
		source_key: &[u8],
		blob: &[u8],
		error_context: impl FnOnce() -> String,
	) -> Result<Arc<DecodedLtx>> {
		if let Some(decoded) = decoded_blobs.get(source_key) {
			return Ok(Arc::clone(decoded));
		}

		let decoded = self
			.decode_ltx_blob_cached(source_key, blob)
			.await
			.with_context(error_context)?;
		decoded_blobs.insert(source_key.to_vec(), Arc::clone(&decoded));

		Ok(decoded)
	}

	async fn decode_ltx_blob_cached(
		&self,
		source_key: &[u8],
		blob: &[u8],
	) -> Result<Arc<DecodedLtx>> {
		if !self.optimization_flags.decoded_ltx_cache {
			return self.decode_ltx_blob_uncached(blob);
		}

		if let Some(entry) = self.decoded_ltx_blobs.get(source_key).await {
			if entry.blob.as_ref() == blob {
				return Ok(Arc::clone(&entry.decoded));
			}
		}

		let decoded = self.decode_ltx_blob_uncached(blob)?;
		self.decoded_ltx_blobs
			.insert(
				source_key.to_vec(),
				DecodedLtxCacheEntry {
					blob: Arc::from(blob),
					decoded: Arc::clone(&decoded),
				},
			)
			.await;

		Ok(decoded)
	}

	fn decode_ltx_blob_uncached(&self, blob: &[u8]) -> Result<Arc<DecodedLtx>> {
		#[cfg(test)]
		self.ltx_decode_count
			.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		Ok(Arc::new(decode_ltx_v3(blob)?))
	}
}

struct GetPagesTxResult {
	head: DBHead,
	pgnos: Vec<u32>,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	pidx_hits: usize,
	pidx_misses: usize,
	stale_pidx_pgnos: BTreeSet<u32>,
}

#[derive(Clone)]
enum PageReadRequest {
	Pages(Vec<u32>),
	Range {
		start_pgno: u32,
		max_pages: u32,
		max_bytes: u64,
	},
}

impl PageReadRequest {
	fn operation(&self) -> &'static str {
		match self {
			PageReadRequest::Pages(_) => "get_pages",
			PageReadRequest::Range { .. } => "get_page_range",
		}
	}

	fn validate(&self) -> Result<()> {
		match self {
			PageReadRequest::Pages(pgnos) => {
				for pgno in pgnos {
					ensure!(*pgno > 0, "get_pages does not accept page 0");
				}
			}
			PageReadRequest::Range {
				start_pgno,
				max_pages,
				max_bytes,
			} => {
				ensure!(*start_pgno > 0, "get_page_range does not accept page 0");
				ensure!(*max_pages > 0, "get_page_range requires max_pages > 0");
				ensure!(*max_bytes > 0, "get_page_range requires max_bytes > 0");
			}
		}

		Ok(())
	}

	fn pidx_lookup_pgnos(&self) -> Vec<u32> {
		match self {
			PageReadRequest::Pages(pgnos) => pgnos.clone(),
			PageReadRequest::Range {
				start_pgno,
				max_pages,
				..
			} => contiguous_pgnos(
				*start_pgno,
				(*max_pages).min(SQLITE_GET_PAGE_RANGE_MAX_PAGES),
			),
		}
	}

	fn pgnos_for_head(&self, head: &DBHead) -> Vec<u32> {
		match self {
			PageReadRequest::Pages(pgnos) => pgnos.clone(),
			PageReadRequest::Range {
				start_pgno,
				max_pages,
				max_bytes,
			} => contiguous_pgnos(
				*start_pgno,
				bounded_range_page_count(head, *max_pages, *max_bytes),
			),
		}
	}
}

fn bounded_range_page_count(head: &DBHead, max_pages: u32, max_bytes: u64) -> u32 {
	let page_cap = max_pages.min(SQLITE_GET_PAGE_RANGE_MAX_PAGES);
	let byte_cap = max_bytes.min(SQLITE_GET_PAGE_RANGE_MAX_BYTES);
	let page_size = u64::from(head.page_size.max(1));
	let byte_page_cap = (byte_cap / page_size).max(1);
	page_cap.min(byte_page_cap.min(u64::from(u32::MAX)) as u32)
}

fn contiguous_pgnos(start_pgno: u32, page_count: u32) -> Vec<u32> {
	let available = u32::MAX - start_pgno + 1;
	let page_count = page_count.min(available);
	(0..page_count).map(|offset| start_pgno + offset).collect()
}

async fn load_delta_history_blobs(
	engine: &SqliteEngine,
	actor_id: &str,
) -> Result<BTreeMap<u64, Vec<u8>>> {
	let delta_chunks = udb::scan_prefix_values(
		&engine.db,
		&engine.subspace,
		engine.op_counter.as_ref(),
		delta_prefix(actor_id),
	)
	.await?;
	let mut delta_blobs = BTreeMap::<u64, Vec<u8>>::new();
	for (delta_key, delta_chunk) in delta_chunks {
		let txid = decode_delta_chunk_txid(actor_id, &delta_key)?;
		delta_blobs
			.entry(txid)
			.or_default()
			.extend_from_slice(&delta_chunk);
	}

	Ok(delta_blobs)
}

fn recover_page_from_delta_history(
	actor_id: &str,
	pgno: u32,
	decoded_blobs: &mut BTreeMap<Vec<u8>, Arc<DecodedLtx>>,
	delta_blobs: &BTreeMap<u64, Vec<u8>>,
) -> Result<Option<Vec<u8>>> {
	for (txid, delta_blob) in delta_blobs.iter().rev() {
		let delta_key = delta_chunk_prefix(actor_id, *txid);
		if !decoded_blobs.contains_key(&delta_key) {
			let decoded = decode_ltx_blob_uncached(delta_blob)
				.with_context(|| format!("decode historical delta blob for page {pgno}"))?;
			decoded_blobs.insert(delta_key.clone(), decoded);
		}

		if let Some(bytes) = decoded_blobs
			.get(&delta_key)
			.and_then(|decoded| decoded.get_page(pgno))
			.map(ToOwned::to_owned)
		{
			return Ok(Some(bytes));
		}
	}

	Ok(None)
}

fn decode_ltx_blob_uncached(blob: &[u8]) -> Result<Arc<DecodedLtx>> {
	Ok(Arc::new(decode_ltx_v3(blob)?))
}

async fn load_delta_blob_tx(
	tx: &universaldb::Transaction,
	subspace: &universaldb::Subspace,
	delta_prefix: &[u8],
) -> Result<Option<Vec<u8>>> {
	let delta_chunks = udb::tx_scan_prefix_values(tx, subspace, delta_prefix).await?;
	if delta_chunks.is_empty() {
		return Ok(None);
	}

	let mut delta_blob = Vec::new();
	for (_, chunk) in delta_chunks {
		delta_blob.extend_from_slice(&chunk);
	}

	Ok(Some(delta_blob))
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == PIDX_TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		PIDX_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::{SQLITE_GET_PAGE_RANGE_MAX_PAGES, decode_db_head};
	use crate::engine::SqliteEngine;
	use crate::error::SqliteStorageError;
	use crate::keys::{delta_chunk_key, meta_key, pidx_delta_key, shard_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::open::OpenConfig;
	use crate::optimization_flags::SqliteOptimizationFlags;
	use crate::test_utils::{assert_op_count, clear_op_count, read_value, test_db};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin, encode_db_head,
	};
	use crate::udb::{WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 4,
			head_txid: 9,
			next_txid: 10,
			materialized_txid: 8,
			db_size_pages: 80,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 123,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
			origin: SqliteOrigin::CreatedOnV2,
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
	}

	fn delta_blob_key(actor_id: &str, txid: u64) -> Vec<u8> {
		delta_chunk_key(actor_id, txid, 0)
	}

	fn encoded_blob(txid: u64, commit: u32, pages: &[(u32, u8)]) -> Vec<u8> {
		let pages = pages
			.iter()
			.map(|(pgno, fill)| DirtyPage {
				pgno: *pgno,
				bytes: page(*fill),
			})
			.collect::<Vec<_>>();

		encode_ltx_v3(LtxHeader::delta(txid, commit, 999), &pages).expect("encode test blob")
	}

	#[tokio::test]
	async fn get_pages_reads_committed_delta_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 5;
		head.next_txid = 6;
		head.materialized_txid = 0;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 5),
					encoded_blob(5, 3, &[(1, 0x11), (2, 0x22), (3, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2, 4]).await?.pages;

		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 4,
					bytes: None,
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn get_page_range_matches_equivalent_get_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 9;
		head.next_txid = 10;
		head.materialized_txid = 8;
		head.db_size_pages = 67;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 9),
					encoded_blob(9, 67, &[(2, 0x22), (3, 0x33)]),
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encoded_blob(8, 67, &[(65, 0x65), (66, 0x66)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 9_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 9_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		let expected = engine
			.get_pages(TEST_ACTOR, 4, vec![2, 3, 4, 5, 6, 7])
			.await?;
		let actual = engine
			.get_page_range(
				TEST_ACTOR,
				4,
				2,
				6,
				u64::from(SQLITE_PAGE_SIZE) * 6,
			)
			.await?;

		assert_eq!(actual.pages, expected.pages);
		assert_eq!(actual.meta, expected.meta);

		Ok(())
	}

	#[tokio::test]
	async fn get_page_range_applies_page_and_byte_caps() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 0;
		head.next_txid = 1;
		head.materialized_txid = 0;
		head.db_size_pages = 400;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&head)?,
			)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		let byte_limited = engine
			.get_page_range(
				TEST_ACTOR,
				4,
				10,
				10,
				u64::from(SQLITE_PAGE_SIZE) * 2,
			)
			.await?;
		assert_eq!(
			byte_limited.pages.iter().map(|page| page.pgno).collect::<Vec<_>>(),
			vec![10, 11]
		);

		let hard_capped = engine
			.get_page_range(TEST_ACTOR, 4, 1, u32::MAX, u64::MAX)
			.await?;
		assert_eq!(
			hard_capped.pages.len(),
			SQLITE_GET_PAGE_RANGE_MAX_PAGES as usize
		);
		assert_eq!(hard_capped.pages.first().map(|page| page.pgno), Some(1));
		assert_eq!(
			hard_capped.pages.last().map(|page| page.pgno),
			Some(SQLITE_GET_PAGE_RANGE_MAX_PAGES)
		);

		Ok(())
	}

	#[tokio::test]
	async fn get_page_range_rejects_invalid_requests_and_generation_mismatch() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&head)?,
			)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		clear_op_count(&engine);

		let page_zero_error = engine
			.get_page_range(TEST_ACTOR, 4, 0, 1, u64::from(SQLITE_PAGE_SIZE))
			.await
			.expect_err("page zero should fail");
		assert!(page_zero_error.to_string().contains("page 0"));

		let zero_pages_error = engine
			.get_page_range(TEST_ACTOR, 4, 1, 0, u64::from(SQLITE_PAGE_SIZE))
			.await
			.expect_err("zero max pages should fail");
		assert!(zero_pages_error.to_string().contains("max_pages"));

		let zero_bytes_error = engine
			.get_page_range(TEST_ACTOR, 4, 1, 1, 0)
			.await
			.expect_err("zero max bytes should fail");
		assert!(zero_bytes_error.to_string().contains("max_bytes"));
		assert_op_count(&engine, 0);

		let generation_error = engine
			.get_page_range(TEST_ACTOR, 99, 1, 1, u64::from(SQLITE_PAGE_SIZE))
			.await
			.expect_err("generation mismatch should fail");
		assert!(generation_error.chain().any(|cause| {
			let msg = cause.to_string();
			msg.contains("did not match open generation") || msg.contains("fence mismatch")
		}));
		assert_op_count(&engine, 0);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_requires_open_before_reading_empty_store() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);

		let error = engine
			.get_pages(TEST_ACTOR, 1, vec![1, 2])
			.await
			.expect_err("read without prior open should fail");
		// `ensure_open` rejects the read before it touches META, so the
		// surfaced error names the lifecycle gate rather than the underlying
		// MetaMissing condition.
		assert_eq!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(&SqliteStorageError::DbNotOpen {
				operation: "get_pages",
			})
		);

		assert!(
			read_value(&engine, meta_key(TEST_ACTOR)).await?.is_none(),
			"read path should not write bootstrap meta"
		);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_batches_delta_and_shard_sources_once() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 9),
					encoded_blob(9, 80, &[(2, 0x24)]),
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encoded_blob(8, 80, &[(65, 0x65), (70, 0x70)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 9_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![2, 65]).await?.pages;

		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x24)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
			]
		);

		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_reuses_decoded_ltx_cache_across_reads() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 9),
					encoded_blob(9, 80, &[(2, 0x24)]),
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encoded_blob(8, 80, &[(65, 0x65), (70, 0x70)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 9_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		engine.reset_ltx_decode_count();
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![2, 65]).await?.pages;
		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x24)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
			]
		);
		assert_eq!(engine.ltx_decode_count(), 2);

		engine.reset_ltx_decode_count();
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![2, 65]).await?.pages;
		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x24)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
			]
		);
		assert_eq!(engine.ltx_decode_count(), 0);

		Ok(())
	}

	#[tokio::test]
	async fn disabled_decoded_ltx_cache_decodes_repeated_reads() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 9;
		head.next_txid = 10;
		head.db_size_pages = 3;
		let flags = SqliteOptimizationFlags {
			decoded_ltx_cache: false,
			..SqliteOptimizationFlags::default()
		};
		let (engine, _compaction_rx) =
			SqliteEngine::new_with_optimization_flags(db, subspace, flags);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 9),
					encoded_blob(9, 3, &[(2, 0x24)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 9_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		engine.reset_ltx_decode_count();
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![2]).await?.pages,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x24)),
			}]
		);
		assert_eq!(engine.ltx_decode_count(), 1);

		engine.reset_ltx_decode_count();
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![2]).await?.pages,
			vec![FetchedPage {
				pgno: 2,
				bytes: Some(page(0x24)),
			}]
		);
		assert_eq!(engine.ltx_decode_count(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_reuses_cached_pidx_without_rescanning() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 4;
		head.next_txid = 5;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 3, &[(3, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		let warmed_pages = engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages;
		assert_eq!(
			warmed_pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		clear_op_count(&engine);

		let pages = engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages;
		assert_eq!(
			pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_falls_back_to_shard_when_cached_pidx_is_stale() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 4;
		head.next_txid = 5;
		head.materialized_txid = 4;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 3, &[(3, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(4, 3, &[(3, 0x44)])),
				WriteOp::delete(delta_blob_key(TEST_ACTOR, 4)),
				WriteOp::delete(pidx_delta_key(TEST_ACTOR, 3)),
			],
		)
		.await?;
		clear_op_count(&engine);

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x44)),
			}]
		);
		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_falls_back_to_shard_when_delta_blob_lacks_cached_page() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 4;
		head.next_txid = 5;
		head.materialized_txid = 4;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 3, &[(3, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(4, 3, &[(3, 0x44)])),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 3, &[(2, 0x22)]),
				),
			],
		)
		.await?;
		clear_op_count(&engine);

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x44)),
			}]
		);

		clear_op_count(&engine);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x44)),
			}]
		);
		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_zero_fills_in_range_pages_when_no_source_exists() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 0;
		head.next_txid = 1;
		head.materialized_txid = 0;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&head)?,
			)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?.pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(vec![0; SQLITE_PAGE_SIZE as usize]),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_rejects_page_zero_and_generation_mismatch() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&head)?,
			)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		clear_op_count(&engine);

		let page_zero_error = engine
			.get_pages(TEST_ACTOR, 4, vec![0])
			.await
			.expect_err("page zero should fail");
		assert!(page_zero_error.to_string().contains("page 0"));
		assert_op_count(&engine, 0);

		let generation_error = engine
			.get_pages(TEST_ACTOR, 99, vec![1])
			.await
			.expect_err("generation mismatch should fail");
		// `ensure_open` surfaces fence mismatches with a message that names the
		// operation and the two generations rather than the older "fence
		// mismatch" wording.
		assert!(generation_error.chain().any(|cause| {
			let msg = cause.to_string();
			msg.contains("did not match open generation") || msg.contains("fence mismatch")
		}));
		// `ensure_open` rejects the mismatched generation before get_pages
		// opens any UDB transaction, so no ops are recorded.
		assert_op_count(&engine, 0);

		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should stay readable"),
		)?;
		assert_eq!(stored_head.generation, 4);

		Ok(())
	}
}
