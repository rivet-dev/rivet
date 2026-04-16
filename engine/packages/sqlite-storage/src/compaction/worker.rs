//! Background compaction worker that schedules shard passes from live PIDX rows.

use std::collections::BTreeSet;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::engine::SqliteEngine;
use crate::keys::pidx_delta_prefix;
use crate::udb;

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const DEFAULT_SHARDS_PER_BATCH: usize = 8;

impl SqliteEngine {
	pub async fn compact_default_batch(&self, actor_id: &str) -> Result<usize> {
		self.compact_worker(actor_id, DEFAULT_SHARDS_PER_BATCH)
			.await
	}

	pub async fn compact_worker(&self, actor_id: &str, shards_per_batch: usize) -> Result<usize> {
		if shards_per_batch == 0 {
			return Ok(0);
		}

		let head = self.load_head(actor_id).await?;
		let pidx_rows = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			pidx_delta_prefix(actor_id),
		)
		.await?;
		let mut shard_ids = BTreeSet::new();

		for (key, _) in pidx_rows {
			let pgno = decode_pidx_pgno(actor_id, &key)?;
			shard_ids.insert(pgno / head.shard_size);
		}

		let mut compacted = 0usize;
		for shard_id in shard_ids.into_iter().take(shards_per_batch) {
			let start = Instant::now();
			if self.compact_shard(actor_id, shard_id).await? {
				self.metrics.observe_compaction_pass(start.elapsed());
				self.metrics.inc_compaction_pass_total();
				compacted += 1;
			}
		}

		Ok(compacted)
	}
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = pidx_delta_prefix(actor_id);
	anyhow::ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	anyhow::ensure!(
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

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use crate::engine::SqliteEngine;
	use crate::keys::{delta_key, meta_key, pidx_delta_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::test_utils::{scan_prefix_values, test_db};
	use crate::types::{
		DBHead, DirtyPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE, SQLITE_SHARD_SIZE,
		SQLITE_VFS_V2_SCHEMA_VERSION,
	};
	use crate::udb::{WriteOp, apply_write_ops};

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
	async fn compact_worker_limits_batch_to_requested_shard_count() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let head = seeded_head();
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut mutations = vec![WriteOp::put(
			meta_key(TEST_ACTOR),
			serde_bare::to_vec(&head)?,
		)];

		for shard_id in 0..9u32 {
			let pgno = shard_id * SQLITE_SHARD_SIZE + 1;
			let txid = u64::from(shard_id) + 1;
			mutations.push(WriteOp::put(
				delta_key(TEST_ACTOR, txid),
				encoded_blob(txid, head.db_size_pages, &[(pgno, txid as u8)]),
			));
			mutations.push(WriteOp::put(
				pidx_delta_key(TEST_ACTOR, pgno),
				txid.to_be_bytes().to_vec(),
			));
		}
		apply_write_ops(
			engine.db.as_ref(),
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
}
