use std::collections::BTreeMap;

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption, error::DatabaseError, options::StreamingMode, utils::IsolationLevel::Serializable,
};

use crate::conveyer::{
	keys::{self, SHARD_SIZE},
	ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3},
	types::{DatabaseBranchId, DirtyPage},
};

use super::helpers::{
	decode_branch_pidx_pgno, decode_branch_shard_id, tracked_entry_size, tx_get_value,
	tx_scan_prefix_values,
};

#[derive(Default)]
pub(super) struct TruncateCleanup {
	pub(super) pidx_clears: Vec<ObservedCleanupRow>,
	/// New pruned shard versions written at the truncating txid. Older shard
	/// versions are retained so pins and PITR coverage can still read
	/// pre-truncate state; reads at or after the truncate resolve to the pruned
	/// version, so regrown pages zero-fill instead of leaking stale bytes.
	pub(super) shard_prune_writes: Vec<(ObservedCleanupRow, Vec<u8>, Vec<u8>)>,
	pub(super) truncated_pgnos: Vec<u32>,
	pub(super) added_bytes: i64,
	pub(super) deleted_bytes: i64,
}

pub(super) struct ObservedCleanupRow {
	pub(super) key: Vec<u8>,
	value: Vec<u8>,
}

pub(super) async fn collect_truncate_cleanup(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	previous_db_size_pages: u32,
	new_db_size_pages: u32,
	truncate_txid: u64,
	now_ms: i64,
) -> Result<TruncateCleanup> {
	if new_db_size_pages >= previous_db_size_pages {
		return Ok(TruncateCleanup::default());
	}

	let mut cleanup = TruncateCleanup::default();
	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id)).await? {
		let pgno = decode_branch_pidx_pgno(branch_id, &key)?;
		if pgno > new_db_size_pages {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.truncated_pgnos.push(pgno);
			cleanup.pidx_clears.push(ObservedCleanupRow { key, value });
		}
	}

	// Keep only the newest version per shard. Rows scan in ascending
	// (shard_id, as_of_txid) order, so the last row per shard wins. The scan
	// must be Serializable: the pruned version asserts that no newer version
	// exists for the shard, so a hot install publishing a new version key
	// between this scan and the commit must conflict instead of being shadowed
	// by a pruned copy built from stale content.
	let mut newest_by_shard = BTreeMap::<u32, (Vec<u8>, Vec<u8>)>::new();
	let boundary_shard_id = new_db_size_pages / SHARD_SIZE;
	let mut shard_scan_start = keys::branch_shard_prefix(branch_id);
	shard_scan_start.extend_from_slice(&boundary_shard_id.to_be_bytes());
	let (_, shard_scan_end) =
		universaldb::tuple::Subspace::from_bytes(keys::branch_shard_prefix(branch_id)).range();
	for (key, value) in
		tx_scan_range_values_serializable(tx, &shard_scan_start, &shard_scan_end).await?
	{
		let shard_id = decode_branch_shard_id(branch_id, &key)?;
		newest_by_shard.insert(shard_id, (key, value));
	}

	for (shard_id, (key, value)) in newest_by_shard {
		let pruned_value =
			prune_truncated_shard_value(&value, new_db_size_pages, truncate_txid, now_ms)
				.context("prune sqlite shard after truncate")?;
		let Some(pruned_value) = pruned_value else {
			continue;
		};

		// Shard versions are not quota-billed: the install publish path writes
		// them without a quota debit, so the pruned copies stay unbilled too.
		let new_key = keys::branch_shard_key(branch_id, shard_id, truncate_txid);
		cleanup
			.shard_prune_writes
			.push((ObservedCleanupRow { key, value }, new_key, pruned_value));
	}

	Ok(cleanup)
}

async fn tx_scan_range_values_serializable(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(start, end).into()
		},
		Serializable,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

pub(super) async fn fence_truncate_cleanup_row(
	tx: &universaldb::Transaction,
	row: &ObservedCleanupRow,
) -> Result<()> {
	let current = tx_get_value(tx, &row.key, Serializable).await?;
	if current.as_deref() != Some(row.value.as_slice()) {
		return Err(DatabaseError::NotCommitted.into());
	}

	Ok(())
}

/// Returns the pruned shard blob to publish at the truncating txid, or `None`
/// when the newest version already has no pages above the new EOF.
fn prune_truncated_shard_value(
	value: &[u8],
	new_db_size_pages: u32,
	truncate_txid: u64,
	now_ms: i64,
) -> Result<Option<Vec<u8>>> {
	let decoded = decode_ltx_v3(value).context("decode sqlite shard for truncate prune")?;
	let original_page_count = decoded.pages.len();
	let live_pages = decoded
		.pages
		.into_iter()
		.filter(|page| page.pgno <= new_db_size_pages)
		.collect::<Vec<DirtyPage>>();
	if live_pages.len() == original_page_count {
		return Ok(None);
	}

	let commit = live_pages.iter().map(|page| page.pgno).max().unwrap_or(1);
	encode_ltx_v3(LtxHeader::delta(truncate_txid, commit, now_ms), &live_pages)
		.context("encode pruned sqlite shard after truncate")
		.map(Some)
}
