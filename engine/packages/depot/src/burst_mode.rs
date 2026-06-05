use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel;

use crate::conveyer::{
	keys,
	types::{CompactionRoot, DatabaseBranchId, decode_compaction_root, decode_db_head},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurstSignal {
	pub head_txid: u64,
	pub compaction_watermark_txid: u64,
	pub lag_txids: u64,
	pub active: bool,
}

pub fn signal_from_txids(head_txid: u64, compaction_watermark_txid: u64) -> BurstSignal {
	let lag_txids = head_txid.saturating_sub(compaction_watermark_txid);

	BurstSignal {
		head_txid,
		compaction_watermark_txid,
		lag_txids,
		active: false,
	}
}

pub async fn read_branch_signal(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: IsolationLevel,
) -> Result<BurstSignal> {
	let informal = tx.informal();
	let head_key = keys::branch_meta_head_key(branch_id);
	let compaction_root_key = keys::branch_compaction_root_key(branch_id);
	let head_fut = informal.get(&head_key, isolation_level);
	let compaction_root_fut = informal.get(&compaction_root_key, isolation_level);
	let (head, compaction_root) = tokio::try_join!(head_fut, compaction_root_fut)?;
	let head_txid = head
		.map(|bytes| decode_db_head(bytes.as_ref()))
		.transpose()
		.context("decode sqlite burst-mode head")?
		.map_or(0, |head| head.head_txid);
	let compaction_root = compaction_root
		.map(|bytes| decode_compaction_root(bytes.as_ref()))
		.transpose()
		.context("decode sqlite burst-mode compaction root")?;

	Ok(read_branch_signal_for_head(
		head_txid,
		compaction_root.as_ref(),
	))
}

pub fn read_branch_signal_for_head(
	head_txid: u64,
	compaction_root: Option<&CompactionRoot>,
) -> BurstSignal {
	let compaction_watermark_txid =
		compaction_root.map_or(head_txid, |root| root.hot_watermark_txid);

	signal_from_txids(head_txid, compaction_watermark_txid)
}

pub fn adjusted_hot_quota_cap(base_cap_bytes: i64, _signal: BurstSignal) -> Result<i64> {
	Ok(base_cap_bytes)
}
