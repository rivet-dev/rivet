use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel;

use crate::{
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS, HOT_BURST_MULTIPLIER,
	pump::{
		keys,
		types::{DatabaseBranchId, decode_db_head},
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurstSignal {
	pub head_txid: u64,
	pub cold_drained_txid: u64,
	pub lag_txids: u64,
	pub active: bool,
}

pub fn signal_from_txids(head_txid: u64, cold_drained_txid: u64) -> BurstSignal {
	let lag_txids = head_txid.saturating_sub(cold_drained_txid);

	BurstSignal {
		head_txid,
		cold_drained_txid,
		lag_txids,
		active: lag_txids >= HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	}
}

pub async fn read_branch_signal(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: IsolationLevel,
) -> Result<BurstSignal> {
	let head_txid = tx
		.informal()
		.get(&keys::branch_meta_head_key(branch_id), isolation_level)
		.await?
		.map(|bytes| decode_db_head(bytes.as_ref()))
		.transpose()
		.context("decode sqlite burst-mode head")?
		.map_or(0, |head| head.head_txid);

	read_branch_signal_for_head(tx, branch_id, head_txid, isolation_level).await
}

pub async fn read_branch_signal_for_head(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head_txid: u64,
	isolation_level: IsolationLevel,
) -> Result<BurstSignal> {
	let cold_drained_txid = tx
		.informal()
		.get(
			&keys::branch_manifest_cold_drained_txid_key(branch_id),
			isolation_level,
		)
		.await?
		.map(|value| decode_u64_be(value.as_ref(), "sqlite burst-mode cold_drained_txid"))
		.transpose()?
		.unwrap_or_default();

	Ok(signal_from_txids(head_txid, cold_drained_txid))
}

pub fn adjusted_hot_quota_cap(base_cap_bytes: i64, signal: BurstSignal) -> Result<i64> {
	if signal.active {
		base_cap_bytes
			.checked_mul(HOT_BURST_MULTIPLIER)
			.context("sqlite burst-mode hot quota cap overflowed")
	} else {
		Ok(base_cap_bytes)
	}
}

fn decode_u64_be(bytes: &[u8], context: &'static str) -> Result<u64> {
	let bytes = <[u8; std::mem::size_of::<u64>()]>::try_from(bytes)
		.map_err(|_| anyhow::anyhow!("{context} had {} bytes", bytes.len()))?;

	Ok(u64::from_be_bytes(bytes))
}
