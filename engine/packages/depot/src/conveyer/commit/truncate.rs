use anyhow::{Context, Result};
use universaldb::{error::DatabaseError, utils::IsolationLevel::Serializable};

use crate::conveyer::{
	keys::{self, SHARD_SIZE},
	ltx::{decode_ltx_v3, encode_ltx_v3},
	types::DatabaseBranchId,
};

use super::helpers::{
	decode_branch_pidx_pgno, decode_branch_shard_id, tracked_entry_size, tx_get_value,
	tx_scan_prefix_values,
};

#[derive(Default)]
pub(super) struct TruncateCleanup {
	pub(super) pidx_clears: Vec<ObservedCleanupRow>,
	pub(super) shard_clears: Vec<ObservedCleanupRow>,
	pub(super) shard_writes: Vec<(ObservedCleanupRow, Vec<u8>)>,
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

	let boundary_shard_id = new_db_size_pages / SHARD_SIZE;
	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_shard_prefix(branch_id)).await? {
		let shard_id = decode_branch_shard_id(branch_id, &key)?;
		if shard_id > boundary_shard_id {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.shard_clears.push(ObservedCleanupRow { key, value });
		} else if shard_id == boundary_shard_id {
			let pruned_value = prune_truncated_shard_value(&value, new_db_size_pages)
				.context("prune sqlite boundary shard after truncate")?;
			if let Some(pruned_value) = pruned_value {
				cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
				let observed = ObservedCleanupRow { key, value };
				if !pruned_value.is_empty() {
					cleanup.added_bytes += tracked_entry_size(&observed.key, &pruned_value)?;
					cleanup.shard_writes.push((observed, pruned_value));
				} else {
					cleanup.shard_clears.push(observed);
				}
			}
		}
	}

	Ok(cleanup)
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

fn prune_truncated_shard_value(value: &[u8], new_db_size_pages: u32) -> Result<Option<Vec<u8>>> {
	let decoded = decode_ltx_v3(value).context("decode sqlite boundary shard")?;
	let original_page_count = decoded.pages.len();
	let live_pages = decoded
		.pages
		.into_iter()
		.filter(|page| page.pgno <= new_db_size_pages)
		.collect::<Vec<_>>();
	if live_pages.len() == original_page_count {
		return Ok(None);
	}
	if live_pages.is_empty() {
		return Ok(Some(Vec::new()));
	}

	encode_ltx_v3(decoded.header, &live_pages)
		.context("encode pruned sqlite boundary shard")
		.map(Some)
}
