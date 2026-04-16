//! Page read paths for sqlite-storage.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use scc::hash_map::Entry;

use crate::engine::SqliteEngine;
use crate::error::SqliteStorageError;
use crate::keys::{delta_key, delta_prefix, meta_key, pidx_delta_prefix, shard_key};
use crate::ltx::{DecodedLtx, decode_ltx_v3};
use crate::page_index::DeltaPageIndex;
use crate::types::{DBHead, FetchedPage};
use crate::udb;

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

impl SqliteEngine {
	pub async fn get_pages(
		&self,
		actor_id: &str,
		generation: u64,
		pgnos: Vec<u32>,
	) -> Result<Vec<FetchedPage>> {
		let start = Instant::now();
		let requested_page_count = pgnos.len();
		for pgno in &pgnos {
			ensure!(*pgno > 0, "get_pages does not accept page 0");
		}

		let pgnos_in_range = pgnos.iter().copied().collect::<Vec<_>>();
		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let cached_pidx = match self.page_indices.get_async(&actor_id).await {
			Some(entry) => Some(
				pgnos_in_range
					.iter()
					.map(|pgno| (*pgno, entry.get().get(*pgno)))
					.collect::<BTreeMap<_, _>>(),
			),
			None => None,
		};
		let tx_result = udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			let cached_pidx = cached_pidx.clone();
			let pgnos_in_range = pgnos_in_range.clone();
			async move {
				let meta_key = meta_key(&actor_id);
				let head =
					if let Some(meta_bytes) = udb::tx_get_value(&tx, &subspace, &meta_key).await? {
						decode_db_head(&meta_bytes)?
					} else {
						ensure!(
							generation == 1,
							SqliteStorageError::MetaMissing {
								operation: "get_pages",
							}
						);
						return Err(SqliteStorageError::MetaMissing {
							operation: "get_pages",
						}
						.into());
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

				let pgnos_in_range = pgnos_in_range
					.into_iter()
					.filter(|pgno| *pgno <= head.db_size_pages)
					.collect::<Vec<_>>();
				if pgnos_in_range.is_empty() {
					return Ok(GetPagesTxResult {
						head,
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
						delta_key(&actor_id, txid)
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
						let mut blob = udb::tx_get_value(&tx, &subspace, &source_key).await?;
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
			loaded_pidx_rows,
			page_sources,
			source_blobs,
			pidx_hits,
			pidx_misses,
			stale_pidx_pgnos,
		} = tx_result;
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
		if page_sources.is_empty() {
			self.metrics
				.observe_get_pages(requested_page_count, start.elapsed());
			return Ok(pgnos
				.into_iter()
				.map(|pgno| FetchedPage {
					pgno,
					bytes: if pgno <= head.db_size_pages {
						Some(vec![0; head.page_size as usize])
					} else {
						None
					},
				})
				.collect());
		}
		let mut decoded_blobs = BTreeMap::new();
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

				if !decoded_blobs.contains_key(source_key) {
					let decoded = decode_ltx_v3(&blob)
						.with_context(|| format!("decode source blob for page {pgno}"))?;
					decoded_blobs.insert(source_key.clone(), decoded);
				}

				bytes = decoded_blobs
					.get(source_key)
					.and_then(|decoded| decoded.get_page(pgno))
					.map(ToOwned::to_owned);
				if bytes.is_none() {
					let shard_source_key = shard_key(&actor_id, pgno / head.shard_size);
					if source_key != &shard_source_key {
						stale_pidx_pgnos.insert(pgno);

						if !decoded_blobs.contains_key(&shard_source_key) {
							if let Some(shard_blob) = udb::get_value(
								self.db.as_ref(),
								&self.subspace,
								self.op_counter.as_ref(),
								shard_source_key.clone(),
							)
							.await?
							{
								let decoded = decode_ltx_v3(&shard_blob).with_context(|| {
									format!("decode shard source blob for stale page {pgno}")
								})?;
								decoded_blobs.insert(shard_source_key.clone(), decoded);
							}
						}

						bytes = decoded_blobs
							.get(&shard_source_key)
							.and_then(|decoded| decoded.get_page(pgno))
							.map(ToOwned::to_owned);
					}
				}
			}
			if bytes.is_none() {
				stale_pidx_pgnos.insert(pgno);
				bytes = recover_page_from_delta_history(self, &actor_id, pgno, &mut decoded_blobs)
					.await?;
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

		Ok(pages)
	}
}

struct GetPagesTxResult {
	head: DBHead,
	loaded_pidx_rows: Option<Vec<(u32, u64)>>,
	page_sources: BTreeMap<u32, Vec<u8>>,
	source_blobs: BTreeMap<Vec<u8>, Vec<u8>>,
	pidx_hits: usize,
	pidx_misses: usize,
	stale_pidx_pgnos: BTreeSet<u32>,
}

fn decode_db_head(bytes: &[u8]) -> Result<DBHead> {
	serde_bare::from_slice(bytes).context("decode sqlite db head")
}

async fn recover_page_from_delta_history(
	engine: &SqliteEngine,
	actor_id: &str,
	pgno: u32,
	decoded_blobs: &mut BTreeMap<Vec<u8>, DecodedLtx>,
) -> Result<Option<Vec<u8>>> {
	let delta_blobs = udb::scan_prefix_values(
		engine.db.as_ref(),
		&engine.subspace,
		engine.op_counter.as_ref(),
		delta_prefix(actor_id),
	)
	.await?;

	for (delta_key, delta_blob) in delta_blobs.into_iter().rev() {
		if !decoded_blobs.contains_key(&delta_key) {
			let decoded = decode_ltx_v3(&delta_blob)
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

	use super::decode_db_head;
	use crate::engine::SqliteEngine;
	use crate::error::SqliteStorageError;
	use crate::keys::{delta_key, meta_key, pidx_delta_key, shard_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::test_utils::{assert_op_count, clear_op_count, read_value, test_db};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION,
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
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(
					delta_key(TEST_ACTOR, 5),
					encoded_blob(5, 3, &[(1, 0x11), (2, 0x22), (3, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2, 4]).await?;

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
	async fn get_pages_requires_takeover_before_reading_empty_store() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);

		let error = engine
			.get_pages(TEST_ACTOR, 1, vec![1, 2])
			.await
			.expect_err("missing meta should require takeover");
		assert_eq!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(&SqliteStorageError::MetaMissing {
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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 9), encoded_blob(9, 80, &[(2, 0x24)])),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encoded_blob(8, 80, &[(65, 0x65), (70, 0x70)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 9_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![2, 65]).await?;

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
	async fn get_pages_reuses_cached_pidx_without_rescanning() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 4;
		head.next_txid = 5;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 3, &[(3, 0x33)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		let warmed_pages = engine.get_pages(TEST_ACTOR, 4, vec![3]).await?;
		assert_eq!(
			warmed_pages,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		clear_op_count(&engine);

		let pages = engine.get_pages(TEST_ACTOR, 4, vec![3]).await?;
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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 3, &[(3, 0x33)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(4, 3, &[(3, 0x44)])),
				WriteOp::delete(delta_key(TEST_ACTOR, 4)),
				WriteOp::delete(pidx_delta_key(TEST_ACTOR, 3)),
			],
		)
		.await?;
		clear_op_count(&engine);

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 3, &[(3, 0x33)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 4_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(4, 3, &[(3, 0x44)])),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 3, &[(2, 0x22)])),
			],
		)
		.await?;
		clear_op_count(&engine);

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x44)),
			}]
		);

		clear_op_count(&engine);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x44)),
			}]
		);
		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn get_pages_recovers_from_older_delta_when_latest_source_is_wrong() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 5;
		head.next_txid = 6;
		head.materialized_txid = 0;
		head.db_size_pages = 3;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 3, &[(3, 0x22)])),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 3, &[(3, 0x33)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x33)),
			}]
		);

		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				delta_key(TEST_ACTOR, 5),
				encoded_blob(5, 3, &[(2, 0x55)]),
			)],
		)
		.await?;
		clear_op_count(&engine);

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
			vec![FetchedPage {
				pgno: 3,
				bytes: Some(page(0x22)),
			}]
		);
		assert_op_count(&engine, 3);

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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				serde_bare::to_vec(&head)?,
			)],
		)
		.await?;

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![3]).await?,
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
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				serde_bare::to_vec(&head)?,
			)],
		)
		.await?;
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
		assert!(
			generation_error
				.chain()
				.any(|cause| cause.to_string().contains("generation fence mismatch"))
		);
		assert_op_count(&engine, 1);

		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should stay readable"),
		)?;
		assert_eq!(stored_head.generation, 4);

		Ok(())
	}
}
