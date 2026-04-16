//! Shard compaction pass that folds live DELTA pages into immutable SHARD blobs.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use scc::hash_map::Entry;

use crate::engine::SqliteEngine;
use crate::keys::{delta_prefix, meta_key, pidx_delta_prefix, shard_key};
use crate::ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3};
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{DBHead, DirtyPage, SQLITE_PAGE_SIZE};
use crate::udb::{self, WriteOp};

const DELTA_TXID_BYTES: usize = std::mem::size_of::<u64>();
const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, PartialEq, Eq)]
struct PidxRow {
	key: Vec<u8>,
	pgno: u32,
	txid: u64,
}

impl SqliteEngine {
	pub async fn compact_shard(&self, actor_id: &str, shard_id: u32) -> Result<bool> {
		let meta_bytes = udb::get_value(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?
		.context("sqlite meta missing for shard compaction")?;
		let mut head = decode_db_head(&meta_bytes)?;

		let shard_start_pgno = shard_id * head.shard_size;
		let shard_end_pgno = shard_start_pgno + head.shard_size.saturating_sub(1);

		let all_pidx_rows = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			pidx_delta_prefix(actor_id),
		)
		.await?
		.into_iter()
		.map(|(key, value)| {
			let pgno = decode_pidx_pgno(actor_id, &key)?;
			let txid = decode_pidx_txid(&value)?;
			Ok(PidxRow { key, pgno, txid })
		})
		.collect::<Result<Vec<_>>>()?;
		let shard_rows = all_pidx_rows
			.iter()
			.filter(|row| row.pgno >= shard_start_pgno && row.pgno <= shard_end_pgno)
			.cloned()
			.collect::<Vec<_>>();
		if shard_rows.is_empty() {
			return Ok(false);
		}

		let delta_entries = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			delta_prefix(actor_id),
		)
		.await?
		.into_iter()
		.map(|(key, value)| {
			let txid = decode_delta_txid(actor_id, &key)?;
			Ok((txid, (key, value)))
		})
		.collect::<Result<BTreeMap<_, _>>>()?;

		let shard_txids = shard_rows
			.iter()
			.map(|row| row.txid)
			.collect::<BTreeSet<_>>();
		let mut blob_keys = Vec::with_capacity(shard_txids.len() + 1);
		let shard_blob_key = shard_key(actor_id, shard_id);
		blob_keys.push(shard_blob_key.clone());
		for txid in &shard_txids {
			blob_keys.push(
				delta_entries
					.get(txid)
					.map(|(key, _)| key.clone())
					.with_context(|| format!("missing delta key for txid {txid}"))?,
			);
		}

		let blob_values = udb::batch_get_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			blob_keys.clone(),
		)
		.await?;
		let blobs = blob_keys
			.into_iter()
			.zip(blob_values)
			.collect::<BTreeMap<_, _>>();
		let delta_keys = delta_entries
			.iter()
			.map(|(txid, (key, _))| (*txid, key.clone()))
			.collect::<BTreeMap<_, _>>();
		let merged_pages = merge_shard_pages(
			&head,
			shard_start_pgno,
			shard_end_pgno,
			&shard_blob_key,
			&blobs,
			&shard_rows,
			&delta_keys,
		)?;
		ensure!(
			!merged_pages.is_empty(),
			"shard {} compaction produced no pages",
			shard_id
		);

		let mut total_refs_by_txid = BTreeMap::<u64, usize>::new();
		for row in &all_pidx_rows {
			*total_refs_by_txid.entry(row.txid).or_default() += 1;
		}
		let mut consumed_refs_by_txid = BTreeMap::<u64, usize>::new();
		for row in &shard_rows {
			*consumed_refs_by_txid.entry(row.txid).or_default() += 1;
		}

		let deleted_delta_txids = delta_keys
			.keys()
			.filter(|txid| {
				let total = total_refs_by_txid.get(txid).copied().unwrap_or(0);
				let consumed = consumed_refs_by_txid.get(txid).copied().unwrap_or(0);
				total <= consumed
			})
			.copied()
			.collect::<BTreeSet<_>>();
		let now_ms = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
			.unwrap_or_default();
		let compaction_lags = deleted_delta_txids
			.iter()
			.filter_map(|txid| delta_entries.get(txid))
			.filter_map(|(_, value)| decode_ltx_v3(value).ok())
			.filter_map(|decoded| {
				let lag_ms = now_ms.checked_sub(decoded.header.timestamp_ms)?;
				Some(lag_ms as f64 / 1000.0)
			})
			.collect::<Vec<_>>();
		head.materialized_txid =
			compute_materialized_txid(&head, delta_entries.keys().copied(), &deleted_delta_txids);

		let shard_commit_txid = shard_rows
			.iter()
			.map(|row| row.txid)
			.max()
			.expect("non-empty shard rows should have a max txid");
		let shard_blob = encode_ltx_v3(
			LtxHeader::delta(shard_commit_txid, head.db_size_pages, head.creation_ts_ms),
			&merged_pages,
		)
		.context("encode compacted shard blob")?;
		let old_meta_size = tracked_storage_entry_size(&meta_key(actor_id), &meta_bytes)
			.expect("meta key should count toward sqlite quota");
		let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(old_meta_size);
		if let Some(existing_shard) = blobs.get(&shard_blob_key).cloned().flatten() {
			usage_without_meta = usage_without_meta.saturating_sub(
				tracked_storage_entry_size(&shard_blob_key, &existing_shard)
					.expect("shard key should count toward sqlite quota"),
			);
		}
		usage_without_meta += tracked_storage_entry_size(&shard_blob_key, &shard_blob)
			.expect("shard key should count toward sqlite quota");
		for row in &shard_rows {
			usage_without_meta = usage_without_meta.saturating_sub(
				tracked_storage_entry_size(&row.key, &row.txid.to_be_bytes())
					.expect("pidx key should count toward sqlite quota"),
			);
		}
		for txid in &deleted_delta_txids {
			if let Some((key, value)) = delta_entries.get(txid) {
				usage_without_meta = usage_without_meta.saturating_sub(
					tracked_storage_entry_size(key, value)
						.expect("delta key should count toward sqlite quota"),
				);
			}
		}
		let (updated_head, encoded_head) =
			encode_db_head_with_usage(actor_id, &head, usage_without_meta)?;
		head = updated_head;

		let mut mutations = Vec::with_capacity(2 + shard_rows.len() + deleted_delta_txids.len());
		mutations.push(WriteOp::put(shard_blob_key.clone(), shard_blob));
		for row in &shard_rows {
			mutations.push(WriteOp::delete(row.key.clone()));
		}
		for txid in &deleted_delta_txids {
			if let Some((key, _)) = delta_entries.get(txid) {
				mutations.push(WriteOp::delete(key.clone()));
			}
		}
		mutations.push(WriteOp::put(meta_key(actor_id), encoded_head));
		udb::apply_write_ops(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			mutations,
		)
		.await?;
		self.metrics.add_compaction_pages_folded(shard_rows.len());
		self.metrics
			.add_compaction_deltas_deleted(deleted_delta_txids.len());
		self.metrics.set_delta_count_from_head(&head);
		for lag_seconds in compaction_lags {
			self.metrics.observe_compaction_lag_seconds(lag_seconds);
		}

		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for row in shard_rows {
					entry.get().remove(row.pgno);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		Ok(true)
	}
}

fn merge_shard_pages(
	head: &DBHead,
	shard_start_pgno: u32,
	shard_end_pgno: u32,
	shard_blob_key: &[u8],
	blobs: &BTreeMap<Vec<u8>, Option<Vec<u8>>>,
	shard_rows: &[PidxRow],
	delta_keys: &BTreeMap<u64, Vec<u8>>,
) -> Result<Vec<DirtyPage>> {
	let mut merged_pages = BTreeMap::<u32, (u64, Vec<u8>)>::new();

	if let Some(shard_blob) = blobs.get(shard_blob_key).cloned().flatten() {
		let decoded = decode_ltx_v3(&shard_blob).context("decode existing shard blob")?;
		for page in decoded.pages {
			if page.pgno >= shard_start_pgno && page.pgno <= shard_end_pgno {
				merged_pages.insert(page.pgno, (head.materialized_txid, page.bytes));
			}
		}
	}

	let shard_txids = shard_rows
		.iter()
		.map(|row| row.txid)
		.collect::<BTreeSet<_>>();
	for txid in shard_txids {
		let delta_key = delta_keys
			.get(&txid)
			.with_context(|| format!("missing delta key for txid {txid}"))?;
		let delta_blob = blobs
			.get(delta_key)
			.cloned()
			.flatten()
			.with_context(|| format!("missing delta blob for txid {txid}"))?;
		let decoded =
			decode_ltx_v3(&delta_blob).with_context(|| format!("decode delta blob {txid}"))?;
		for page in decoded.pages {
			ensure!(
				page.bytes.len() == SQLITE_PAGE_SIZE as usize,
				"page {} had {} bytes, expected {}",
				page.pgno,
				page.bytes.len(),
				SQLITE_PAGE_SIZE
			);
			if page.pgno >= shard_start_pgno && page.pgno <= shard_end_pgno {
				merged_pages.insert(page.pgno, (txid, page.bytes));
			}
		}
	}

	Ok(merged_pages
		.into_iter()
		.map(|(pgno, (_, bytes))| DirtyPage { pgno, bytes })
		.collect())
}

fn compute_materialized_txid(
	head: &DBHead,
	remaining_delta_txids: impl IntoIterator<Item = u64>,
	deleted_delta_txids: &BTreeSet<u64>,
) -> u64 {
	let next_live_txid = remaining_delta_txids
		.into_iter()
		.filter(|txid| *txid > head.materialized_txid && !deleted_delta_txids.contains(txid))
		.min();

	match next_live_txid {
		Some(txid) => txid.saturating_sub(1).max(head.materialized_txid),
		None => head.head_txid,
	}
}

fn decode_db_head(bytes: &[u8]) -> Result<DBHead> {
	serde_bare::from_slice(bytes).context("decode sqlite db head")
}

fn decode_delta_txid(actor_id: &str, key: &[u8]) -> Result<u64> {
	let prefix = delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"delta key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == DELTA_TXID_BYTES,
		"delta key suffix had {} bytes, expected {}",
		suffix.len(),
		DELTA_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		suffix
			.try_into()
			.context("delta key suffix should decode as u64")?,
	))
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
	use crate::keys::{delta_key, meta_key, pidx_delta_key, pidx_delta_prefix, shard_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::test_utils::{read_value, scan_prefix_values, test_db};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION,
	};
	use crate::udb::{WriteOp, apply_write_ops, test_hooks};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 4,
			head_txid: 5,
			next_txid: 6,
			materialized_txid: 0,
			db_size_pages: 129,
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

	async fn actual_tracked_usage(engine: &SqliteEngine) -> Result<u64> {
		Ok(scan_prefix_values(engine, vec![0x02])
			.await?
			.into_iter()
			.filter_map(|(key, value)| tracked_storage_entry_size(&key, &value))
			.sum())
	}

	async fn rewrite_meta_with_actual_usage(engine: &SqliteEngine) -> Result<DBHead> {
		let head = decode_db_head(
			&read_value(engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist before rewrite"),
		)?;
		let usage_without_meta = actual_tracked_usage(engine).await?.saturating_sub(
			tracked_storage_entry_size(
				&meta_key(TEST_ACTOR),
				&read_value(engine, meta_key(TEST_ACTOR))
					.await?
					.expect("meta should exist before rewrite"),
			)
			.expect("meta key should count toward sqlite quota"),
		);
		let (head, meta_bytes) = encode_db_head_with_usage(TEST_ACTOR, &head, usage_without_meta)?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(TEST_ACTOR), meta_bytes)],
		)
		.await?;
		Ok(head)
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
	async fn compact_worker_folds_five_deltas_into_one_shard() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 5;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 1), encoded_blob(1, 5, &[(1, 0x11)])),
				WriteOp::put(delta_key(TEST_ACTOR, 2), encoded_blob(2, 5, &[(2, 0x22)])),
				WriteOp::put(delta_key(TEST_ACTOR, 3), encoded_blob(3, 5, &[(3, 0x33)])),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 5, &[(4, 0x44)])),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 5, &[(5, 0x55)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 3_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 4), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 5), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		let _ = engine.get_or_load_pidx(TEST_ACTOR).await?;

		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);
		assert!(
			scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);

		let stored_head: DBHead = serde_bare::from_slice(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after compaction"),
		)?;
		assert_eq!(stored_head.materialized_txid, 5);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2, 3, 4, 5]).await?;
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
					pgno: 3,
					bytes: Some(page(0x33)),
				},
				FetchedPage {
					pgno: 4,
					bytes: Some(page(0x44)),
				},
				FetchedPage {
					pgno: 5,
					bytes: Some(page(0x55)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_prefers_latest_delta_over_old_shard_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 2;
		head.next_txid = 3;
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(
					shard_key(TEST_ACTOR, 0),
					encoded_blob(0.max(1), 2, &[(1, 0x10), (2, 0x20)]),
				),
				WriteOp::put(delta_key(TEST_ACTOR, 1), encoded_blob(1, 2, &[(1, 0x11)])),
				WriteOp::put(
					delta_key(TEST_ACTOR, 2),
					encoded_blob(2, 2, &[(1, 0x22), (2, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);

		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2]).await?;
		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x33)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_keeps_quota_usage_in_sync() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 2, &[(1, 0x10)])),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 2, &[(2, 0x20)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		rewrite_meta_with_actual_usage(&engine).await?;
		let before_usage = actual_tracked_usage(&engine).await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);

		let after_usage = actual_tracked_usage(&engine).await?;
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after compaction"),
		)?;

		assert_eq!(stored_head.sqlite_storage_used, after_usage);
		assert!(after_usage <= before_usage);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_retries_cleanly_after_store_error() -> Result<()> {
		const FAIL_ACTOR: &str = "test-actor-compaction-failure";

		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(FAIL_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(FAIL_ACTOR, 4), encoded_blob(4, 2, &[(1, 0x10)])),
				WriteOp::put(delta_key(FAIL_ACTOR, 5), encoded_blob(5, 2, &[(2, 0x20)])),
				WriteOp::put(pidx_delta_key(FAIL_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(FAIL_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		let head = decode_db_head(
			&read_value(&engine, meta_key(FAIL_ACTOR))
				.await?
				.expect("meta should exist before quota rewrite"),
		)?;
		let usage_without_meta = actual_tracked_usage(&engine).await?.saturating_sub(
			tracked_storage_entry_size(
				&meta_key(FAIL_ACTOR),
				&read_value(&engine, meta_key(FAIL_ACTOR))
					.await?
					.expect("meta should exist before quota rewrite"),
			)
			.expect("meta key should count toward sqlite quota"),
		);
		let (_, meta_bytes) = encode_db_head_with_usage(FAIL_ACTOR, &head, usage_without_meta)?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(FAIL_ACTOR), meta_bytes)],
		)
		.await?;
		let before_usage = actual_tracked_usage(&engine).await?;
		let _guard = test_hooks::fail_next_apply_write_ops_matching(meta_key(FAIL_ACTOR));

		let error = engine
			.compact_shard(FAIL_ACTOR, 0)
			.await
			.expect_err("injected compaction store error should fail the pass");
		let error_text = format!("{error:#}");

		assert!(error_text.contains("InjectedStoreError"), "{error_text}");
		assert_eq!(actual_tracked_usage(&engine).await?, before_usage);
		assert!(
			read_value(&engine, delta_key(FAIL_ACTOR, 4))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_key(FAIL_ACTOR, 5))
				.await?
				.is_some()
		);
		assert_eq!(
			scan_prefix_values(&engine, pidx_delta_prefix(FAIL_ACTOR))
				.await?
				.len(),
			2
		);

		assert!(engine.compact_shard(FAIL_ACTOR, 0).await?);
		assert_eq!(
			engine.get_pages(FAIL_ACTOR, 4, vec![1, 2]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x10)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x20)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_handles_multi_shard_delta_across_three_passes() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 1;
		head.next_txid = 2;
		head.db_size_pages = 129;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(
					delta_key(TEST_ACTOR, 1),
					encoded_blob(1, 129, &[(1, 0x11), (65, 0x65), (129, 0x81)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 65), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 129),
					1_u64.to_be_bytes().to_vec(),
				),
			],
		)
		.await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);

		assert!(engine.compact_shard(TEST_ACTOR, 1).await?);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);

		assert!(engine.compact_shard(TEST_ACTOR, 2).await?);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1, 65, 129]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
				FetchedPage {
					pgno: 129,
					bytes: Some(page(0x81)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_is_idempotent() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(delta_key(TEST_ACTOR, 4), encoded_blob(4, 2, &[(1, 0x10)])),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 2, &[(2, 0x20)])),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 0);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1, 2]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x10)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x20)),
				},
			]
		);

		Ok(())
	}
}
