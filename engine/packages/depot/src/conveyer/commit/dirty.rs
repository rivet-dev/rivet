use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Serializable;

use crate::{
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	conveyer::{
		keys, quota,
		types::{
			CompactionRoot, DatabaseBranchId, SqliteCmpDirty, decode_compaction_root,
			decode_db_head, decode_sqlite_cmp_dirty, encode_sqlite_cmp_dirty,
		},
		udb,
	},
	workflows::compaction::DeltasAvailable,
};

use super::helpers::tx_get_value;

pub(super) async fn admit_deltas_available(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head_txid: u64,
	compaction_root: Option<&CompactionRoot>,
	fallback_cold_watermark_txid: u64,
	now_ms: i64,
	last_signal_at_ms: Option<i64>,
) -> Result<Option<DeltasAvailable>> {
	if !has_actionable_lag(head_txid, compaction_root, fallback_cold_watermark_txid) {
		return Ok(None);
	}

	let dirty_key = keys::sqlite_cmp_dirty_key(branch_id);
	let previous_dirty = tx_get_value(tx, &dirty_key, Serializable)
		.await?
		.as_deref()
		.map(decode_sqlite_cmp_dirty)
		.transpose()
		.context("decode sqlite compaction dirty marker")?;
	let dirty = SqliteCmpDirty {
		observed_head_txid: head_txid,
		updated_at_ms: now_ms,
	};
	let encoded_dirty =
		encode_sqlite_cmp_dirty(dirty.clone()).context("encode sqlite compaction dirty marker")?;
	tx.informal().set(&dirty_key, &encoded_dirty);

	let first_dirty_writer = previous_dirty.is_none();
	let throttled_signal_due = last_signal_at_ms.is_none_or(|last_signal_at_ms| {
		now_ms.saturating_sub(last_signal_at_ms)
			>= i64::try_from(quota::TRIGGER_THROTTLE_MS).unwrap_or(i64::MAX)
	});
	if first_dirty_writer || throttled_signal_due {
		Ok(Some(DeltasAvailable {
			database_branch_id: branch_id,
			observed_head_txid: dirty.observed_head_txid,
			dirty_updated_at_ms: dirty.updated_at_ms,
		}))
	} else {
		Ok(None)
	}
}

fn has_actionable_lag(
	head_txid: u64,
	compaction_root: Option<&CompactionRoot>,
	fallback_cold_watermark_txid: u64,
) -> bool {
	let hot_watermark_txid = compaction_root.map_or(0, |root| root.hot_watermark_txid);
	let cold_watermark_txid = compaction_root.map_or(fallback_cold_watermark_txid, |root| {
		root.cold_watermark_txid
	});
	let hot_lag = head_txid.saturating_sub(hot_watermark_txid);
	let cold_lag = head_txid.saturating_sub(cold_watermark_txid);

	hot_lag >= quota::COMPACTION_DELTA_THRESHOLD || cold_lag >= HOT_BURST_COLD_LAG_THRESHOLD_TXIDS
}

pub async fn clear_sqlite_cmp_dirty_if_observed_idle(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	observed_dirty: SqliteCmpDirty,
) -> Result<bool> {
	db.run(move |tx| {
		let observed_dirty = observed_dirty.clone();
		async move {
			let dirty_key = keys::sqlite_cmp_dirty_key(branch_id);
			let expected_dirty = encode_sqlite_cmp_dirty(observed_dirty.clone())
				.context("encode observed sqlite compaction dirty marker")?;
			let Some(current_dirty) = tx_get_value(&tx, &dirty_key, Serializable).await? else {
				return Ok(false);
			};
			if current_dirty != expected_dirty {
				return Ok(false);
			}
			if branch_has_actionable_lag(&tx, branch_id).await? {
				return Ok(false);
			}

			udb::compare_and_clear(&tx, &dirty_key, &expected_dirty);
			Ok(true)
		}
	})
	.await
}

async fn branch_has_actionable_lag(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<bool> {
	let head_txid = tx_get_value(tx, &keys::branch_meta_head_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()
		.context("decode sqlite db head for dirty clear")?
		.map_or(0, |head| head.head_txid);
	let compaction_root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for dirty clear")?
	.context(
		"sqlite compaction root missing for dirty clear; \
			workflow manager must publish CMP/root before clearing dirty marker",
	)?;
	Ok(has_actionable_lag(head_txid, Some(&compaction_root), 0))
}
