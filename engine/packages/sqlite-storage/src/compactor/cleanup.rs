//! PITR cleanup helpers for retained checkpoints and leaked refcounts.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use rivet_pools::NodeId;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::{
	admin::{OpStatus, decode_admin_op_record},
	pump::{
		keys,
		quota,
		types::{
			Checkpoints, RetentionConfig, decode_checkpoint_meta, decode_checkpoints,
			decode_delta_meta, encode_checkpoint_meta, encode_checkpoints, encode_delta_meta,
		},
	},
};

use super::metrics;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckpointCleanupOutcome {
	pub checkpoints_deleted: u64,
	pub bytes_freed: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefcountLeakOutcome {
	pub checkpoint_refs_reset: u64,
	pub delta_refs_reset: u64,
}

pub async fn cleanup_old_checkpoints(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	retention_config: RetentionConfig,
	now_ms: i64,
) -> Result<CheckpointCleanupOutcome> {
	cleanup_old_checkpoints_with_metric_context(
		udb,
		actor_id,
		retention_config,
		now_ms,
		None,
		None,
	)
	.await
}

pub(crate) async fn cleanup_old_checkpoints_with_metric_context(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	retention_config: RetentionConfig,
	now_ms: i64,
	namespace_id: Option<Id>,
	actor_name: Option<String>,
) -> Result<CheckpointCleanupOutcome> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let actor_name = actor_name.clone();

		async move {
			let checkpoints_key = keys::meta_checkpoints_key(&actor_id);
			let Some(existing_checkpoints_bytes) =
				tx.informal().get(&checkpoints_key, Serializable).await?
			else {
				return Ok(CheckpointCleanupOutcome::default());
			};
			let mut checkpoints = decode_checkpoints(&existing_checkpoints_bytes)
				.context("decode sqlite checkpoints for cleanup")?;
			let Some(latest_txid) = checkpoints.entries.iter().map(|entry| entry.ckp_txid).max()
			else {
				return Ok(CheckpointCleanupOutcome::default());
			};

			if let (Some(namespace_id), Some(actor_name)) = (namespace_id, actor_name.as_ref()) {
				let pinned_count = checkpoints
					.entries
					.iter()
					.filter(|entry| entry.refcount > 0)
					.count();
				if pinned_count > 0 {
					let namespace_tx = tx.with_subspace(namespace::keys::subspace());
					namespace::keys::metric::inc(
						&namespace_tx,
						namespace_id,
						namespace::keys::metric::Metric::SqliteCheckpointPinned(actor_name.clone()),
						i64::try_from(pinned_count)
							.context("sqlite pinned checkpoint count exceeded i64")?,
					);
				}
			}

			let cutoff_ms = retention_cutoff(now_ms, retention_config.retention_ms);
			let mut delete_txids = Vec::new();
			checkpoints.entries.retain(|entry| {
				let should_delete = entry.ckp_txid != latest_txid
					&& entry.refcount == 0
					&& retention_expired(entry.taken_at_ms, cutoff_ms, retention_config.retention_ms);
				if should_delete {
					delete_txids.push(entry.ckp_txid);
				}
				!should_delete
			});

			if delete_txids.is_empty() {
				return Ok(CheckpointCleanupOutcome::default());
			}

			let mut prefix_bytes_freed = 0i64;
			for ckp_txid in &delete_txids {
				let prefix = keys::checkpoint_prefix(&actor_id, *ckp_txid);
				prefix_bytes_freed += scan_tracked_prefix_bytes(&tx, &prefix).await?;
				let (begin, end) = prefix_range(&prefix);
				tx.informal().clear_range(&begin, &end);
			}

			let encoded_checkpoints =
				encode_checkpoints(checkpoints).context("encode sqlite checkpoints after cleanup")?;
			let old_index_bytes = tracked_entry_size(&checkpoints_key, &existing_checkpoints_bytes)?;
			let new_index_bytes = tracked_entry_size(&checkpoints_key, &encoded_checkpoints)?;
			let pitr_delta = -prefix_bytes_freed + new_index_bytes - old_index_bytes;

			tx.informal().set(&checkpoints_key, &encoded_checkpoints);
			if pitr_delta != 0 {
				quota::atomic_add_pitr(&tx, &actor_id, pitr_delta);
			}

			Ok(CheckpointCleanupOutcome {
				checkpoints_deleted: delete_txids.len() as u64,
				bytes_freed: prefix_bytes_freed,
			})
		}
	})
	.await
}

pub async fn detect_refcount_leaks(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	now_ms: i64,
	lease_ttl_ms: u64,
) -> Result<RefcountLeakOutcome> {
	detect_refcount_leaks_with_node_id(udb, actor_id, now_ms, lease_ttl_ms, NodeId::new()).await
}

pub(crate) async fn detect_refcount_leaks_with_node_id(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	now_ms: i64,
	lease_ttl_ms: u64,
	node_id: NodeId,
) -> Result<RefcountLeakOutcome> {
	let node_id = node_id.to_string();
	let outcome = udb
		.run(move |tx| {
			let actor_id = actor_id.clone();

			async move {
				if has_live_admin_op(&tx, &actor_id).await? {
					return Ok(RefcountLeakOutcome::default());
				}

				let leak_cutoff_ms =
					retention_cutoff(now_ms, lease_ttl_ms.saturating_mul(10));
				let checkpoint_refs_reset =
					reset_checkpoint_refcount_leaks(&tx, &actor_id, leak_cutoff_ms).await?;
				let delta_refs_reset =
					reset_delta_refcount_leaks(&tx, &actor_id, leak_cutoff_ms).await?;

				Ok(RefcountLeakOutcome {
					checkpoint_refs_reset,
					delta_refs_reset,
				})
			}
		})
		.await?;

	if outcome.checkpoint_refs_reset > 0 {
		metrics::SQLITE_CHECKPOINT_REFCOUNT_LEAK_TOTAL
			.with_label_values(&[node_id.as_str(), "checkpoint"])
			.inc_by(outcome.checkpoint_refs_reset);
	}
	if outcome.delta_refs_reset > 0 {
		metrics::SQLITE_CHECKPOINT_REFCOUNT_LEAK_TOTAL
			.with_label_values(&[node_id.as_str(), "delta"])
			.inc_by(outcome.delta_refs_reset);
	}

	Ok(outcome)
}

async fn reset_checkpoint_refcount_leaks(
	tx: &universaldb::Transaction,
	actor_id: &str,
	leak_cutoff_ms: i64,
) -> Result<u64> {
	let checkpoints_key = keys::meta_checkpoints_key(actor_id);
	let existing_checkpoints_bytes = tx.informal().get(&checkpoints_key, Serializable).await?;
	let mut checkpoints = existing_checkpoints_bytes
		.as_deref()
		.map(|value| decode_checkpoints(value))
		.transpose()
		.context("decode sqlite checkpoints for refcount leak detection")?
		.unwrap_or(Checkpoints {
			entries: Vec::new(),
		});
	let mut entries_by_txid = checkpoints
		.entries
		.iter()
		.enumerate()
		.map(|(idx, entry)| (entry.ckp_txid, idx))
		.collect::<BTreeMap<_, _>>();

	let mut reset_count = 0u64;
	let checkpoint_entries = checkpoints.entries.clone();
	for entry in checkpoint_entries {
		let key = keys::checkpoint_meta_key(actor_id, entry.ckp_txid);
		let Some(meta_bytes) = tx.informal().get(&key, Serializable).await? else {
			continue;
		};
		let mut meta =
			decode_checkpoint_meta(&meta_bytes).context("decode sqlite checkpoint meta")?;
		if meta.refcount == 0 || meta.taken_at_ms >= leak_cutoff_ms {
			continue;
		}

		meta.refcount = 0;
		tx.informal()
			.set(&key, &encode_checkpoint_meta(meta).context("encode sqlite checkpoint meta")?);
		if let Some(idx) = entries_by_txid.remove(&entry.ckp_txid) {
			checkpoints.entries[idx].refcount = 0;
		}
		reset_count += 1;
	}

	if reset_count > 0 {
		tx.informal().set(
			&checkpoints_key,
			&encode_checkpoints(checkpoints).context("encode sqlite checkpoints")?,
		);
	}

	Ok(reset_count)
}

async fn reset_delta_refcount_leaks(
	tx: &universaldb::Transaction,
	actor_id: &str,
	leak_cutoff_ms: i64,
) -> Result<u64> {
	let mut reset_count = 0u64;
	for (key, value) in tx_scan_prefix_values(tx, &keys::delta_prefix(actor_id)).await? {
		let txid = keys::decode_delta_chunk_txid(actor_id, &key)?;
		if key != keys::delta_meta_key(actor_id, txid) {
			continue;
		}

		let mut meta = decode_delta_meta(&value).context("decode sqlite delta meta")?;
		if meta.refcount == 0 || meta.taken_at_ms >= leak_cutoff_ms {
			continue;
		}

		meta.refcount = 0;
		tx.informal()
			.set(&key, &encode_delta_meta(meta).context("encode sqlite delta meta")?);
		reset_count += 1;
	}

	Ok(reset_count)
}

async fn has_live_admin_op(tx: &universaldb::Transaction, actor_id: &str) -> Result<bool> {
	for (_key, value) in tx_scan_prefix_values(tx, &keys::meta_admin_op_prefix(actor_id)).await? {
		let record = decode_admin_op_record(&value).context("decode sqlite admin op record")?;
		if record.actor_id == actor_id
			&& matches!(record.status, OpStatus::Pending | OpStatus::InProgress)
		{
			return Ok(true);
		}
	}

	Ok(false)
}

fn retention_cutoff(now_ms: i64, retention_ms: u64) -> i64 {
	now_ms.saturating_sub(i64::try_from(retention_ms).unwrap_or(i64::MAX))
}

fn retention_expired(taken_at_ms: i64, cutoff_ms: i64, retention_ms: u64) -> bool {
	retention_ms == 0 || taken_at_ms < cutoff_ms
}

async fn scan_tracked_prefix_bytes(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<i64> {
	tx_scan_prefix_values(tx, prefix)
		.await?
		.iter()
		.map(|(key, value)| tracked_entry_size(key, value))
		.sum()
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
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

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}

fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(prefix.to_vec()).range()
}
