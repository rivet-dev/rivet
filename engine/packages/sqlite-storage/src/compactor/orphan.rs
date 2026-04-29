use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption, tuple,
	options::StreamingMode,
	utils::IsolationLevel::Serializable,
};

use crate::{
	admin::{AdminOpRecord, OpStatus, decode_admin_op_record, encode_admin_op_record},
	pump::keys,
};

use super::metrics;

pub const ORPHAN_THRESHOLD_MS: i64 = 30_000;

pub async fn scan_for_orphans(udb: Arc<universaldb::Database>, now_ms: i64) -> Result<u64> {
	scan_for_orphans_with_threshold(udb, now_ms, ORPHAN_THRESHOLD_MS).await
}

pub async fn scan_for_orphans_with_threshold(
	udb: Arc<universaldb::Database>,
	now_ms: i64,
	orphan_threshold_ms: i64,
) -> Result<u64> {
	udb.run(move |tx| async move {
		let mut orphaned = 0_u64;
		let informal = tx.informal();
		let sqlite_subspace = universaldb::Subspace::from(tuple::Subspace::from_bytes(vec![
			keys::SQLITE_SUBSPACE_PREFIX,
		]));
		let mut stream = informal.get_ranges_keyvalues(
			RangeOption {
				mode: StreamingMode::WantAll,
				..RangeOption::from(&sqlite_subspace)
			},
			Serializable,
		);

		while let Some(entry) = stream.try_next().await? {
			let key = entry.key();
			if !key_has_admin_op_suffix(key) {
				continue;
			}

			let mut record = decode_admin_op_record(entry.value())
				.context("decode sqlite admin op record during orphan scan")?;
			if should_orphan(&record, now_ms, orphan_threshold_ms) {
				record.status = OpStatus::Orphaned;
				record.last_progress_at_ms =
					now_ms.max(record.last_progress_at_ms.saturating_add(1));
				informal.set(
					key,
					&encode_admin_op_record(record).context("encode orphaned sqlite admin op")?,
				);
				orphaned = orphaned.saturating_add(1);
				metrics::SQLITE_ADMIN_OP_ORPHANED_TOTAL.inc();
			}
		}

		Ok(orphaned)
	})
	.await
}

fn should_orphan(record: &AdminOpRecord, now_ms: i64, orphan_threshold_ms: i64) -> bool {
	matches!(record.status, OpStatus::Pending)
		&& record.holder_id.is_none()
		&& now_ms.saturating_sub(record.created_at_ms) > orphan_threshold_ms
}

fn key_has_admin_op_suffix(key: &[u8]) -> bool {
	key.windows(b"/META/admin_op/".len())
		.any(|window| window == b"/META/admin_op/")
}
