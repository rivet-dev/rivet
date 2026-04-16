//! Takeover handling for writer fencing and preload.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use anyhow::{Context, Result, ensure};

use crate::engine::SqliteEngine;
use crate::error::SqliteStorageError;
use crate::keys::{delta_key, delta_prefix, meta_key, pidx_delta_prefix, shard_key, stage_prefix};
use crate::ltx::decode_ltx_v3;
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{DBHead, FetchedPage, SQLITE_MAX_DELTA_BYTES, SqliteMeta};
use crate::udb::{self, WriteOp};

pub const DEFAULT_PRELOAD_MAX_BYTES: usize = 1024 * 1024;

const DELTA_TXID_BYTES: usize = std::mem::size_of::<u64>();
const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnoRange {
	pub start: u32,
	pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeoverConfig {
	pub now_ms: i64,
	pub preload_pgnos: Vec<u32>,
	pub preload_ranges: Vec<PgnoRange>,
	pub max_total_bytes: usize,
}

impl TakeoverConfig {
	pub fn new(now_ms: i64) -> Self {
		Self {
			now_ms,
			preload_pgnos: Vec::new(),
			preload_ranges: Vec::new(),
			max_total_bytes: DEFAULT_PRELOAD_MAX_BYTES,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeoverResult {
	pub generation: u64,
	pub meta: SqliteMeta,
	pub preloaded_pages: Vec<FetchedPage>,
}

impl SqliteEngine {
	pub async fn takeover(&self, actor_id: &str, config: TakeoverConfig) -> Result<TakeoverResult> {
		let start = Instant::now();
		let meta_bytes = udb::get_value(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?;
		let mut live_pidx = BTreeMap::new();
		let mut mutations = Vec::new();
		let mut should_schedule_compaction = false;
		let mut recovered_orphans = 0usize;
		let usage_without_meta = if let Some(meta_bytes) = meta_bytes.as_ref() {
			let head = decode_db_head(meta_bytes)?;
			head.sqlite_storage_used.saturating_sub(
				tracked_storage_entry_size(&meta_key(actor_id), meta_bytes)
					.expect("meta key should count toward sqlite quota"),
			)
		} else {
			0
		};

		let head = if let Some(meta_bytes) = meta_bytes.clone() {
			let head = decode_db_head(&meta_bytes)?;
			let recovery_plan = self
				.build_recovery_plan(actor_id, &head, &mut live_pidx)
				.await?;
			should_schedule_compaction = recovery_plan.live_delta_count >= 32;
			let tracked_deleted_bytes = recovery_plan.tracked_deleted_bytes;
			recovered_orphans = recovery_plan.orphan_count;
			mutations.extend(recovery_plan.mutations);
			let mut head = DBHead {
				generation: head.generation + 1,
				..head
			};
			head.sqlite_storage_used = usage_without_meta.saturating_sub(tracked_deleted_bytes);
			head
		} else {
			DBHead::new(config.now_ms)
		};

		let (head, encoded_head) =
			encode_db_head_with_usage(actor_id, &head, head.sqlite_storage_used)?;
		mutations.push(WriteOp::put(meta_key(actor_id), encoded_head));

		let actor_id_for_tx = actor_id.to_string();
		let expected_meta_bytes = meta_bytes.clone();
		let takeover_mutations = mutations.clone();
		let subspace = self.subspace.clone();
		udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let expected_meta_bytes = expected_meta_bytes.clone();
			let takeover_mutations = takeover_mutations.clone();
			let subspace = subspace.clone();
			async move {
				let current_meta = udb::tx_get_value(&tx, &subspace, &meta_key(&actor_id)).await?;
				if current_meta != expected_meta_bytes {
					tracing::error!(
						actor_id = %actor_id,
						"meta changed during takeover, concurrent writer detected"
					);
					return Err(SqliteStorageError::ConcurrentTakeover.into());
				}

				for op in &takeover_mutations {
					match op {
						WriteOp::Put(key, value) => {
							udb::tx_write_value(&tx, &subspace, key, value)?
						}
						WriteOp::Delete(key) => udb::tx_delete_value(&tx, &subspace, key),
					}
				}

				Ok(())
			}
		})
		.await?;
		if should_schedule_compaction {
			let _ = self.compaction_tx.send(actor_id.to_string());
		}
		self.metrics.add_recovery_orphans_cleaned(recovered_orphans);
		self.metrics.set_delta_count_from_head(&head);

		self.page_indices.remove_async(&actor_id.to_string()).await;

		let preloaded_pages = self
			.preload_pages(actor_id, &head, &live_pidx, &config)
			.await?;
		let meta = SqliteMeta::from((head.clone(), SQLITE_MAX_DELTA_BYTES));
		self.metrics.observe_takeover(start.elapsed());

		Ok(TakeoverResult {
			generation: head.generation,
			meta,
			preloaded_pages,
		})
	}

	async fn build_recovery_plan(
		&self,
		actor_id: &str,
		head: &DBHead,
		live_pidx: &mut BTreeMap<u32, u64>,
	) -> Result<RecoveryPlan> {
		let delta_rows = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			delta_prefix(actor_id),
		)
		.await?;
		let stage_rows = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			stage_prefix(actor_id),
		)
		.await?;
		let pidx_rows = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			pidx_delta_prefix(actor_id),
		)
		.await?;

		let mut live_delta_txids = BTreeSet::new();
		let mut mutations = Vec::new();
		let mut tracked_deleted_bytes = 0u64;

		for (key, value) in delta_rows {
			let txid = decode_delta_txid(actor_id, &key)?;
			if txid > head.head_txid {
				tracked_deleted_bytes += tracked_storage_entry_size(&key, &value)
					.expect("delta key should count toward sqlite quota");
				mutations.push(WriteOp::delete(key));
			} else {
				live_delta_txids.insert(txid);
			}
		}

		for (key, _) in stage_rows {
			mutations.push(WriteOp::delete(key));
		}

		for (key, value) in pidx_rows {
			let pgno = decode_pidx_pgno(actor_id, &key)?;
			let txid = decode_pidx_txid(&value)?;

			if txid > head.head_txid || !live_delta_txids.contains(&txid) {
				tracked_deleted_bytes += tracked_storage_entry_size(&key, &value)
					.expect("pidx key should count toward sqlite quota");
				mutations.push(WriteOp::delete(key));
			} else {
				live_pidx.insert(pgno, txid);
			}
		}
		let orphan_count = mutations.len();

		Ok(RecoveryPlan {
			mutations,
			live_delta_count: live_delta_txids.len(),
			orphan_count,
			tracked_deleted_bytes,
		})
	}

	async fn preload_pages(
		&self,
		actor_id: &str,
		head: &DBHead,
		live_pidx: &BTreeMap<u32, u64>,
		config: &TakeoverConfig,
	) -> Result<Vec<FetchedPage>> {
		let requested = collect_preload_pgnos(config);
		let mut sources = BTreeMap::new();

		for pgno in &requested {
			if *pgno == 0 || *pgno > head.db_size_pages {
				continue;
			}

			let key = if let Some(txid) = live_pidx.get(pgno) {
				delta_key(actor_id, *txid)
			} else {
				shard_key(actor_id, *pgno / head.shard_size)
			};
			sources.insert(key, None);
		}

		if !sources.is_empty() {
			let keys = sources.keys().cloned().collect::<Vec<_>>();
			let values = udb::batch_get_values(
				self.db.as_ref(),
				&self.subspace,
				self.op_counter.as_ref(),
				keys.clone(),
			)
			.await?;
			for (key, value) in keys.into_iter().zip(values) {
				sources.insert(key, value);
			}
		}

		let mut decoded_pages = BTreeMap::new();
		let mut total_bytes = 0usize;
		let mut preloaded_pages = Vec::with_capacity(requested.len());

		for pgno in requested {
			if pgno == 0 || pgno > head.db_size_pages {
				preloaded_pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let source_key = if let Some(txid) = live_pidx.get(&pgno) {
				delta_key(actor_id, *txid)
			} else {
				shard_key(actor_id, pgno / head.shard_size)
			};

			let page_bytes = match sources.get(&source_key).cloned().flatten() {
				Some(blob) => {
					let cached = decoded_pages.contains_key(&source_key);
					if !cached {
						let decoded_ltx = decode_ltx_v3(&blob)
							.with_context(|| format!("decode preload blob for page {pgno}"))?;
						decoded_pages.insert(source_key.clone(), decoded_ltx.pages);
					}

					decoded_pages.get(&source_key).and_then(|pages| {
						pages
							.iter()
							.find(|page| page.pgno == pgno)
							.map(|page| page.bytes.clone())
					})
				}
				None => None,
			};

			match page_bytes {
				Some(bytes) if pgno == 1 || total_bytes + bytes.len() <= config.max_total_bytes => {
					total_bytes += bytes.len();
					preloaded_pages.push(FetchedPage {
						pgno,
						bytes: Some(bytes),
					});
				}
				Some(_) | None => {
					preloaded_pages.push(FetchedPage { pgno, bytes: None });
				}
			}
		}

		Ok(preloaded_pages)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryPlan {
	mutations: Vec<WriteOp>,
	live_delta_count: usize,
	orphan_count: usize,
	tracked_deleted_bytes: u64,
}

fn collect_preload_pgnos(config: &TakeoverConfig) -> Vec<u32> {
	let mut requested = BTreeSet::from([1]);
	for pgno in &config.preload_pgnos {
		if *pgno > 0 {
			requested.insert(*pgno);
		}
	}

	for range in &config.preload_ranges {
		for pgno in range.start..range.end {
			if pgno > 0 {
				requested.insert(pgno);
			}
		}
	}

	requested.into_iter().collect()
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
	use tokio::sync::mpsc::error::TryRecvError;

	use super::{PgnoRange, TakeoverConfig};
	use crate::commit::CommitStageRequest;
	use crate::engine::SqliteEngine;
	use crate::keys::{delta_key, meta_key, pidx_delta_key, shard_key, stage_key, stage_prefix};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::test_utils::{
		checkpoint_test_db, read_value, reopen_test_db, scan_prefix_values, test_db,
		test_db_with_path,
	};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_MAX_DELTA_BYTES,
		SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION,
	};
	use crate::udb::{WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 1,
			head_txid: 3,
			next_txid: 4,
			materialized_txid: 0,
			db_size_pages: 4,
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

	fn encoded_blob(txid: u64, pgno: u32, fill: u8) -> Vec<u8> {
		encode_ltx_v3(
			LtxHeader::delta(txid, pgno, 999),
			&[DirtyPage {
				pgno,
				bytes: page(fill),
			}],
		)
		.expect("encode test ltx blob")
	}

	#[tokio::test]
	async fn takeover_on_empty_store_creates_meta_and_page_one_placeholder() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let result = engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(777))
			.await?;

		assert_eq!(result.generation, 1);
		assert_eq!(result.meta.generation, 1);
		assert_eq!(result.meta.head_txid, 0);
		assert_eq!(result.meta.max_delta_bytes, SQLITE_MAX_DELTA_BYTES);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: None,
			}]
		);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		let stored_meta = read_value(&engine, meta_key(TEST_ACTOR))
			.await?
			.expect("meta should exist");
		let head: DBHead = serde_bare::from_slice(&stored_meta)?;
		assert_eq!(head.generation, 1);
		assert_eq!(head.creation_ts_ms, 777);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_on_existing_meta_bumps_generation_and_preloads_page_one() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(1, 1, 0x2a)),
			],
		)
		.await?;
		let result = engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(888))
			.await?;

		assert_eq!(result.generation, 2);
		assert_eq!(result.meta.generation, 2);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x2a)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn preload_returns_requested_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 70;
		head.head_txid = 7;
		head.next_txid = 8;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(
					delta_key(TEST_ACTOR, 7),
					encode_ltx_v3(
						LtxHeader::delta(7, 70, 999),
						&[
							DirtyPage {
								pgno: 1,
								bytes: page(0x11),
							},
							DirtyPage {
								pgno: 2,
								bytes: page(0x22),
							},
						],
					)?,
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encode_ltx_v3(
						LtxHeader::delta(6, 70, 888),
						&[DirtyPage {
							pgno: 65,
							bytes: page(0x65),
						}],
					)?,
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 7_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 7_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		let mut config = TakeoverConfig::new(1_234);
		config.preload_pgnos = vec![65];
		config.preload_ranges.push(PgnoRange { start: 2, end: 3 });

		let result = engine.takeover(TEST_ACTOR, config).await?;
		assert_eq!(
			result.preloaded_pages,
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
					pgno: 65,
					bytes: Some(page(0x65)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_bumps_generation() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(1, 1, 0x2a)),
			],
		)
		.await?;

		let result = engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(888))
			.await?;

		assert_eq!(result.generation, 2);
		assert_eq!(result.meta.generation, 2);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_cleans_orphans_and_stale_pidx_entries() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&seeded_head())?),
				WriteOp::put(delta_key(TEST_ACTOR, 2), encoded_blob(2, 1, 0x11)),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 2, 0x55)),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 99_u64.to_be_bytes().to_vec()),
				WriteOp::put(stage_key(TEST_ACTOR, 42, 0), vec![1, 2, 3]),
				WriteOp::put(stage_key(TEST_ACTOR, 42, 1), vec![4, 5, 6]),
			],
		)
		.await?;
		let result = engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(999))
			.await?;

		assert_eq!(result.generation, 2);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x11)),
			}]
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 2))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 3))
				.await?
				.is_none()
		);
		assert!(
			scan_prefix_values(&engine, stage_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_cleans_orphan_deltas() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&seeded_head())?),
				WriteOp::put(delta_key(TEST_ACTOR, 2), encoded_blob(2, 1, 0x11)),
				WriteOp::put(delta_key(TEST_ACTOR, 5), encoded_blob(5, 2, 0x55)),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(999))
			.await?;

		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 2))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_cleans_orphan_stages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), serde_bare::to_vec(&seeded_head())?),
				WriteOp::put(stage_key(TEST_ACTOR, 42, 0), vec![1, 2, 3]),
				WriteOp::put(stage_key(TEST_ACTOR, 42, 1), vec![4, 5, 6]),
			],
		)
		.await?;

		engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(999))
			.await?;

		assert!(
			scan_prefix_values(&engine, stage_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_recovers_from_checkpointed_mid_commit_stage_state() -> Result<()> {
		let (db, subspace, _db_path) = test_db_with_path().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), subspace.clone());
		let head = DBHead {
			head_txid: 0,
			next_txid: 1,
			db_size_pages: 0,
			..seeded_head()
		};
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
		engine
			.commit_stage(
				TEST_ACTOR,
				CommitStageRequest {
					generation: head.generation,
					stage_id: 77,
					chunk_idx: 0,
					dirty_pages: vec![DirtyPage {
						pgno: 1,
						bytes: page(0x44),
					}],
					is_last: true,
				},
			)
			.await?;
		let checkpoint_path = checkpoint_test_db(engine.db.as_ref())?;
		drop(engine);
		drop(db);

		let reopened_db = reopen_test_db(&checkpoint_path).await?;
		let (recovered_engine, _compaction_rx) = SqliteEngine::new(reopened_db, subspace);
		let result = recovered_engine
			.takeover(TEST_ACTOR, TakeoverConfig::new(2_222))
			.await?;
		let stored_head: DBHead = serde_bare::from_slice(
			&read_value(&recovered_engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should still exist after recovery"),
		)?;

		assert_eq!(result.generation, head.generation + 1);
		assert_eq!(result.meta.head_txid, 0);
		assert_eq!(stored_head.head_txid, 0);
		assert_eq!(stored_head.next_txid, 1);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: None,
			}]
		);
		assert!(
			scan_prefix_values(&recovered_engine, stage_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);
		assert!(
			read_value(&recovered_engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn takeover_schedules_compaction_when_delta_threshold_is_met() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 32;
		head.next_txid = 33;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let mut mutations = vec![WriteOp::put(
			meta_key(TEST_ACTOR),
			serde_bare::to_vec(&head)?,
		)];
		for txid in 1..=32_u64 {
			mutations.push(WriteOp::put(delta_key(TEST_ACTOR, txid), vec![txid as u8]));
		}
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			mutations,
		)
		.await?;
		let mut config = TakeoverConfig::new(1111);
		config.preload_ranges.push(PgnoRange { start: 2, end: 4 });

		let result = engine.takeover(TEST_ACTOR, config).await?;

		assert_eq!(result.generation, 2);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert_eq!(
			result.preloaded_pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: None,
				},
				FetchedPage {
					pgno: 2,
					bytes: None,
				},
				FetchedPage {
					pgno: 3,
					bytes: None,
				},
			]
		);

		Ok(())
	}
}
