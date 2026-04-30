use std::sync::Arc;

use anyhow::Result;
use sqlite_storage::{
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
	burst_mode,
	keys::{branch_manifest_cold_drained_txid_key, branch_meta_head_key},
	types::{ActorBranchId, DBHead, encode_db_head},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-burst-mode-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn head_with_branch(branch_id: ActorBranchId, head_txid: u64) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages: 1,
		post_apply_checksum: 0,
		branch_id,
		#[cfg(debug_assertions)]
		generation: 0,
	}
}

#[tokio::test]
async fn burst_signal_is_derived_from_fdb_cold_lag() -> Result<()> {
	let db = test_db().await?;
	let branch_id = ActorBranchId::new_v4();
	db.run(move |tx| async move {
		tx.informal().set(
			&branch_meta_head_key(branch_id),
			&encode_db_head(head_with_branch(branch_id, HOT_BURST_COLD_LAG_THRESHOLD_TXIDS))?,
		);
		tx.informal()
			.set(&branch_manifest_cold_drained_txid_key(branch_id), &0_u64.to_be_bytes());
		Ok(())
	})
	.await?;

	let active = db
		.run(move |tx| async move {
			burst_mode::read_branch_signal(&tx, branch_id, Snapshot).await
		})
		.await?;
	assert!(active.active);
	assert_eq!(active.lag_txids, HOT_BURST_COLD_LAG_THRESHOLD_TXIDS);

	db.run(move |tx| async move {
		tx.informal().set(
			&branch_manifest_cold_drained_txid_key(branch_id),
			&HOT_BURST_COLD_LAG_THRESHOLD_TXIDS.to_be_bytes(),
		);
		Ok(())
	})
	.await?;

	let recovered = db
		.run(move |tx| async move {
			burst_mode::read_branch_signal(&tx, branch_id, Snapshot).await
		})
		.await?;
	assert!(!recovered.active);
	assert_eq!(recovered.lag_txids, 0);

	Ok(())
}
