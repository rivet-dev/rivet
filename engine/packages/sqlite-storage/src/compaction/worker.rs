//! Background compaction worker that schedules shard passes from live PIDX rows.

use std::collections::BTreeSet;
use std::time::Instant;

use anyhow::Result;

use super::shard::{load_delta_entries, load_pidx_rows};
use crate::engine::SqliteEngine;

const DEFAULT_SHARDS_PER_BATCH: usize = 8;

impl SqliteEngine {
	pub async fn compact_default_batch(&self, actor_id: &str) -> Result<usize> {
		self.compact_worker(actor_id, DEFAULT_SHARDS_PER_BATCH)
			.await
	}

	/// Schedules shard passes from the PIDX and DELTA tables.
	///
	/// Scans PIDX and DELTA once and shares the results with every per-shard pass so an
	/// N-shard batch performs a single PIDX scan plus a single DELTA scan. When a shard
	/// pass succeeds its consumed PIDX rows and deleted DELTA txids are removed from the
	/// in-memory view so subsequent shards compute correct ref counts and do not try to
	/// delete DELTA chunks another shard already removed.
	pub async fn compact_worker(&self, actor_id: &str, shards_per_batch: usize) -> Result<usize> {
		if shards_per_batch == 0 {
			return Ok(0);
		}
		let actor_lock = self.actor_op_lock(actor_id).await;
		let _actor_guard = actor_lock.lock().await;

		let head = self.load_head(actor_id).await?;
		let mut pidx_rows = load_pidx_rows(self, actor_id).await?;
		let mut delta_entries = load_delta_entries(self, actor_id).await?;

		let shard_ids = pidx_rows
			.iter()
			.map(|row| row.pgno / head.shard_size)
			.collect::<BTreeSet<_>>();

		let mut compacted = 0usize;
		for shard_id in shard_ids.into_iter().take(shards_per_batch) {
			let start = Instant::now();
			if let Some(outcome) = self
				.compact_shard_preloaded(actor_id, shard_id, &head, &pidx_rows, &delta_entries)
				.await?
			{
				pidx_rows.retain(|row| !outcome.consumed_pidx_pgnos.contains(&row.pgno));
				for txid in &outcome.deleted_delta_txids {
					delta_entries.remove(txid);
				}
				self.metrics.observe_compaction_pass(start.elapsed());
				self.metrics.inc_compaction_pass_total();
				compacted += 1;
			}
		}

		Ok(compacted)
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use crate::engine::SqliteEngine;
	use crate::keys::{delta_chunk_key, meta_key, pidx_delta_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::test_utils::{clear_op_count, scan_prefix_values, test_db};
	use crate::types::{
		DBHead, DirtyPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE,
		SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin, encode_db_head, new_db_head,
	};
	use crate::udb::{self, WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 4,
			head_txid: 9,
			next_txid: 10,
			materialized_txid: 0,
			db_size_pages: 577,
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
	async fn compact_worker_limits_batch_to_requested_shard_count() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut mutations = vec![WriteOp::put(
			meta_key(TEST_ACTOR),
			encode_db_head(&head)?,
		)];

		for shard_id in 0..9u32 {
			let pgno = shard_id * SQLITE_SHARD_SIZE + 1;
			let txid = u64::from(shard_id) + 1;
			mutations.push(WriteOp::put(
				delta_blob_key(TEST_ACTOR, txid),
				encoded_blob(txid, head.db_size_pages, &[(pgno, txid as u8)]),
			));
			mutations.push(WriteOp::put(
				pidx_delta_key(TEST_ACTOR, pgno),
				txid.to_be_bytes().to_vec(),
			));
		}
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			mutations,
		)
		.await?;
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 8);

		let remaining_pidx =
			scan_prefix_values(&engine, crate::keys::pidx_delta_prefix(TEST_ACTOR)).await?;
		assert_eq!(remaining_pidx.len(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_scans_pidx_and_delta_once_per_batch() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut mutations = vec![WriteOp::put(
			meta_key(TEST_ACTOR),
			encode_db_head(&head)?,
		)];

		// Seed 8 single-page shards so one compact_worker call triggers all 8 shard passes.
		for shard_id in 0..8u32 {
			let pgno = shard_id * SQLITE_SHARD_SIZE + 1;
			let txid = u64::from(shard_id) + 1;
			mutations.push(WriteOp::put(
				delta_blob_key(TEST_ACTOR, txid),
				encoded_blob(txid, head.db_size_pages, &[(pgno, txid as u8)]),
			));
			mutations.push(WriteOp::put(
				pidx_delta_key(TEST_ACTOR, pgno),
				txid.to_be_bytes().to_vec(),
			));
		}
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			mutations,
		)
		.await?;

		clear_op_count(&engine);
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 8);

		// Worker structure after US-062:
		//   1 load_head (META get_value)
		//   1 PIDX scan for the whole batch
		//   1 DELTA scan for the whole batch
		//   N shards × (shard blob get_value + atomic write) = 2 ops per shard
		//
		// Before US-062 this was 1 + N × (PIDX scan + DELTA scan + shard get_value +
		// atomic write) = 1 + 4N ops, with N full PIDX and N full DELTA scans per batch.
		let final_ops = udb::op_count(&engine.op_counter);
		assert_eq!(
			final_ops,
			3 + 2 * 8,
			"compact_worker should do 1 load_head + 1 PIDX scan + 1 DELTA scan + 2N per-shard ops"
		);
		Ok(())
	}
}
