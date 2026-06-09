//! Snapshot-target coverage fence.
//!
//! # Why fork alignment exists
//!
//! Delta retention and fork resolution are two halves of one design. A naive
//! system retains every DELTA chunk for the whole PITR window so a fork or
//! restore can reconstruct *any* historical txid. That couples storage cost to
//! the retention window and forces reclaim to prove, per shard, that coverage
//! exists before it may delete anything.
//!
//! Instead we make the delta rule trivial: a DELTA is reclaimable once its txid
//! is at or below the hot watermark, with no per-shard or PIDX proof. This is
//! sound only because hot compaction's install already published shard coverage
//! for every *covered txid* at or below the watermark (the watermark advance and
//! the shard publish are one atomic transaction). So "below the watermark" means
//! "reconstructable from the newest SHARD version at or below that txid" for the
//! covered points, and nothing else below the watermark is reconstructable.
//!
//! Fork alignment is the constraint that keeps that trade honest: a fork, pin,
//! or restore may only target a txid that is either
//!
//! - above the watermark (deltas still exist; the pin makes the next install
//!   stage shard coverage for that exact txid), or
//! - an already-covered txid at or below the watermark: the watermark itself, a
//!   retained `PITR_INTERVAL` representative, or an existing `DB_PIN`.
//!
//! A versionstamp target that lands between covered points is snapped down to
//! the newest covered point at or below it (see `snap_covered_target`). Reads
//! resolve "newest SHARD version at or below the cap", so a snapped fork reads
//! exactly the covered snapshot. Without this fence a fork could pin a txid
//! whose deltas reclaim just deleted, reading silent zero-fill or stale bytes.
//!
//! # No cross-node clock dependency
//!
//! The covered points are *recorded state*, not a clock computation. Hot
//! compaction writes a `PITR_INTERVAL` row per database branch per interval
//! bucket holding a concrete `(txid, versionstamp, wall_clock_ms, expires_at_ms)`
//! commit, and pins are concrete `(txid, versionstamp)` rows. Alignment then
//! compares **UDB versionstamps** (monotonic, globally ordered commit
//! tokens) against those recorded rows. It does not compare wall-clock times
//! across machines, so it needs no clock synchronization.
//!
//! Wall-clock appears in exactly two single-node, fenced places: hot compaction
//! buckets commits into PITR intervals by their own recorded `wall_clock_ms`,
//! and this fence plus `snap_covered_target` skip interval rows whose
//! `expires_at_ms` has passed the local `now_ms`. Both reads are Serializable,
//! so a fork racing reclaim either conflicts and retries or observes a
//! consistent snapshot; the worst a skewed clock does is admit or reject a fork
//! a few minutes early or late, never resolve it to wrong data.
//!
//! Every check reads Serializable so a concurrent install advancing the
//! watermark conflicts with the fencing transaction instead of racing past it.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Serializable;

use super::{
	history_pin, keys, pitr_interval,
	types::{DatabaseBranchId, decode_compaction_root},
};

pub async fn snapshot_txid_is_resolvable(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<bool> {
	let watermark_txid = tx
		.informal()
		.get(&keys::branch_compaction_root_key(branch_id), Serializable)
		.await?
		.map(Vec::<u8>::from)
		.as_deref()
		.map(decode_compaction_root)
		.transpose()
		.context("decode sqlite compaction root for snapshot fence")?
		.map(|root| root.hot_watermark_txid)
		.unwrap_or(0);
	if txid >= watermark_txid {
		return Ok(true);
	}

	// Expired rows are about to lose their commit islands to reclaim, so they
	// are not deterministic targets even while still present, matching
	// snap_covered_target.
	let now_ms = now_ms()?;
	let interval_rows =
		pitr_interval::scan_pitr_interval_coverage(tx, branch_id, Serializable).await?;
	if interval_rows
		.iter()
		.any(|(_, coverage)| coverage.txid == txid && coverage.expires_at_ms > now_ms)
	{
		return Ok(true);
	}

	let pins = history_pin::read_db_history_pins(tx, branch_id, Serializable).await?;

	Ok(pins.iter().any(|pin| pin.at_txid == txid))
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?
		.as_millis();
	i64::try_from(millis).context("current timestamp exceeded i64 milliseconds")
}
