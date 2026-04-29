//! Single-shot commit path for the stateless sqlite-storage pump.

use std::{collections::BTreeSet, time::Duration};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::pump::{
	ActorDb,
	keys::{self, SHARD_SIZE},
	ltx::{LtxHeader, encode_ltx_v3},
	metrics,
	quota,
	types::{DBHead, DirtyPage, decode_db_head, decode_meta_compact, encode_db_head},
};

const DELTA_CHUNK_BYTES: usize = 10_000;

impl ActorDb {
	pub async fn commit(
		&self,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
	) -> Result<()> {
		let node_id = self.node_id.to_string();
		let labels = &[node_id.as_str()];
		let _timer = metrics::SQLITE_PUMP_COMMIT_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT
			.with_label_values(labels)
			.observe(dirty_pages.len() as f64);

		let cached_storage_used = *self.storage_used.lock();
		let cache_was_warm = !self.cache.lock().range(0, u32::MAX).is_empty();
		let actor_id = self.actor_id.clone();
		let dirty_pages_for_tx = dirty_pages.clone();

		let result = self
			.udb
			.run(move |tx| {
				let actor_id = actor_id.clone();
				let dirty_pages = dirty_pages_for_tx.clone();

				async move {
					let head_key = keys::meta_head_key(&actor_id);
					let (head_bytes, storage_used) = if let Some(storage_used) = cached_storage_used {
						(tx_get_value(&tx, &head_key, Serializable).await?, storage_used)
					} else {
						let quota_fut = quota::read(&tx, &actor_id);
						let head_fut = tx_get_value(&tx, &head_key, Serializable);
						let (head_bytes, storage_used) = tokio::try_join!(head_fut, quota_fut)?;
						(head_bytes, storage_used)
					};

					let previous_head = head_bytes
						.as_deref()
						.map(decode_db_head)
						.transpose()
						.context("decode current sqlite db head")?;
					let materialized_txid = tx_get_value(&tx, &keys::meta_compact_key(&actor_id), Snapshot)
						.await?
						.as_deref()
						.map(decode_meta_compact)
						.transpose()
						.context("decode sqlite compact meta for trigger")?
						.map_or(0, |compact| compact.materialized_txid);
					let previous_db_size_pages =
						previous_head.as_ref().map_or(db_size_pages, |head| head.db_size_pages);
					let txid = match previous_head.as_ref() {
						Some(head) => head
							.head_txid
							.checked_add(1)
							.context("sqlite head txid overflowed")?,
						None => 1,
					};

					let truncate_cleanup =
						collect_truncate_cleanup(&tx, &actor_id, previous_db_size_pages, db_size_pages)
							.await?;

					let encoded_delta = encode_ltx_v3(
						LtxHeader::delta(txid, db_size_pages, now_ms),
						&dirty_pages,
					)
					.context("encode commit delta")?;
					let delta_chunks = encoded_delta
						.chunks(DELTA_CHUNK_BYTES)
						.enumerate()
						.map(|(chunk_idx, chunk)| {
							let chunk_idx = u32::try_from(chunk_idx)
								.context("delta chunk index exceeded u32")?;
							Ok((keys::delta_chunk_key(&actor_id, txid, chunk_idx), chunk.to_vec()))
						})
						.collect::<Result<Vec<_>>>()?;

					let new_head = DBHead {
						head_txid: txid,
						db_size_pages,
						#[cfg(debug_assertions)]
						generation: previous_head.as_ref().map_or(0, |head| head.generation),
					};
					let encoded_head = encode_db_head(new_head).context("encode new sqlite db head")?;
					let txid_bytes = txid.to_be_bytes();
					let dirty_pgnos = dirty_pages
						.iter()
						.map(|page| page.pgno)
						.collect::<BTreeSet<_>>();

					let added_bytes = tracked_entry_size(&head_key, &encoded_head)?
						+ delta_chunks
							.iter()
							.map(|(key, value)| tracked_entry_size(key, value))
							.sum::<Result<i64>>()?
						+ dirty_pgnos
							.iter()
							.map(|pgno| {
								tracked_entry_size(&keys::pidx_delta_key(&actor_id, *pgno), &txid_bytes)
							})
							.sum::<Result<i64>>()?;
					let removed_bytes = head_bytes
						.as_ref()
						.map_or(Ok(0), |bytes| tracked_entry_size(&head_key, bytes))?
						+ truncate_cleanup.deleted_bytes;
					let quota_delta = added_bytes
						.checked_sub(removed_bytes)
						.context("sqlite commit quota delta overflowed i64")?;
					let would_be = storage_used
						.checked_add(quota_delta)
						.context("sqlite commit quota check overflowed i64")?;

					quota::cap_check(would_be)?;

					for (key, value) in &delta_chunks {
						tx.informal().set(key, value);
					}
					for pgno in &dirty_pgnos {
						tx.informal()
							.set(&keys::pidx_delta_key(&actor_id, *pgno), &txid_bytes);
					}
					for key in &truncate_cleanup.pidx_keys {
						tx.informal().clear(key);
					}
					for key in &truncate_cleanup.shard_keys {
						tx.informal().clear(key);
					}
					tx.informal().set(&head_key, &encoded_head);
					if quota_delta != 0 {
						quota::atomic_add(&tx, &actor_id, quota_delta);
					}

					Ok(CommitTxResult {
						txid,
						materialized_txid,
						dirty_pgnos,
						truncated_pgnos: truncate_cleanup.truncated_pgnos,
						added_bytes,
						storage_used: would_be,
					})
				}
			})
			.await?;

		*self.storage_used.lock() = Some(result.storage_used);
		*self.commit_bytes_since_rollup.lock() += u64::try_from(result.added_bytes)
			.context("commit added bytes should be non-negative")?;

		if cache_was_warm {
			let cache = self.cache.lock();
			for pgno in result.truncated_pgnos {
				cache.remove(pgno);
			}
			for pgno in result.dirty_pgnos {
				cache.insert(pgno, result.txid);
			}
		}

		self.publish_compact_trigger_if_needed(result.txid, result.materialized_txid);

		Ok(())
	}

	fn publish_compact_trigger_if_needed(&self, head_txid: u64, materialized_txid: u64) {
		let Some(delta_count) = head_txid.checked_sub(materialized_txid) else {
			return;
		};
		if delta_count < quota::COMPACTION_DELTA_THRESHOLD {
			return;
		}

		let now = tokio::time::Instant::now();
		let should_publish = {
			let mut last_trigger_at = self.last_trigger_at.lock();
			let should_publish = last_trigger_at.is_none_or(|last| {
				now.duration_since(last) >= Duration::from_millis(quota::TRIGGER_THROTTLE_MS)
					|| now.duration_since(last)
						> Duration::from_millis(quota::TRIGGER_MAX_SILENCE_MS)
			});
			if should_publish {
				*last_trigger_at = Some(now);
			}
			should_publish
		};

		if should_publish {
			let (commit_bytes_since_rollup, read_bytes_since_rollup) =
				self.take_metering_snapshot();
			crate::compactor::publish_compact_payload_with_node_id(
				&self.ups,
				crate::compactor::SqliteCompactPayload {
					actor_id: self.actor_id.clone(),
					namespace_id: None,
					actor_name: None,
					commit_bytes_since_rollup,
					read_bytes_since_rollup,
				},
				self.node_id,
			);
		}
	}
}

struct CommitTxResult {
	txid: u64,
	materialized_txid: u64,
	dirty_pgnos: BTreeSet<u32>,
	truncated_pgnos: Vec<u32>,
	added_bytes: i64,
	storage_used: i64,
}

#[derive(Default)]
struct TruncateCleanup {
	pidx_keys: Vec<Vec<u8>>,
	shard_keys: Vec<Vec<u8>>,
	truncated_pgnos: Vec<u32>,
	deleted_bytes: i64,
}

async fn collect_truncate_cleanup(
	tx: &universaldb::Transaction,
	actor_id: &str,
	previous_db_size_pages: u32,
	new_db_size_pages: u32,
) -> Result<TruncateCleanup> {
	if new_db_size_pages >= previous_db_size_pages {
		return Ok(TruncateCleanup::default());
	}

	let mut cleanup = TruncateCleanup::default();
	for (key, value) in tx_scan_prefix_values(tx, &keys::pidx_delta_prefix(actor_id)).await? {
		let pgno = decode_pidx_pgno(actor_id, &key)?;
		if pgno > new_db_size_pages {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.truncated_pgnos.push(pgno);
			cleanup.pidx_keys.push(key);
		}
	}

	for (key, value) in tx_scan_prefix_values(tx, &keys::shard_prefix(actor_id)).await? {
		let shard_id = decode_shard_id(actor_id, &key)?;
		if shard_id.saturating_mul(SHARD_SIZE) > new_db_size_pages {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.shard_keys.push(key);
		}
	}

	Ok(cleanup)
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, isolation_level)
		.await?
		.map(Vec::<u8>::from))
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::pidx_delta_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("pidx key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("pidx key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_shard_id(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::shard_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("shard key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("shard key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}
