mod common;

use anyhow::Result;
use depot::{
	burst_mode,
	keys::{branch_compaction_root_key, branch_meta_head_key},
	types::{CompactionRoot, DBHead, DatabaseBranchId, encode_compaction_root, encode_db_head},
};
use universaldb::utils::IsolationLevel::Snapshot;

fn head_with_branch(branch_id: DatabaseBranchId, head_txid: u64) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages: 1,
		post_apply_checksum: 0,
		branch_id,
	}
}

#[tokio::test]
async fn burst_signal_is_inactive_without_cold_storage() -> Result<()> {
	common::test_matrix("depot-burst-mode", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = DatabaseBranchId::new_v4();
			let head_txid = 1_024;
			db.txn("test_depotburst_mode", move |tx| async move {
				tx.informal().set(
					&branch_meta_head_key(branch_id),
					&encode_db_head(head_with_branch(branch_id, head_txid))?,
				);
				tx.informal().set(
					&branch_compaction_root_key(branch_id),
					&encode_compaction_root(CompactionRoot {
						schema_version: 1,
						manifest_generation: 1,
						hot_watermark_txid: 0,
						cold_watermark_txid: 0,
						cold_watermark_versionstamp: [0; 16],
					})?,
				);
				Ok(())
			})
			.await?;

			let active = db
				.txn("test_depotburst_mode", move |tx| async move {
					burst_mode::read_branch_signal(&tx, branch_id, Snapshot).await
				})
				.await?;
			assert!(!active.active);
			assert_eq!(active.lag_txids, 0);
			assert_eq!(active.compaction_watermark_txid, head_txid);

			db.txn("test_depotburst_mode", move |tx| async move {
				tx.informal().set(
					&branch_compaction_root_key(branch_id),
					&encode_compaction_root(CompactionRoot {
						schema_version: 1,
						manifest_generation: 2,
						hot_watermark_txid: head_txid,
						cold_watermark_txid: head_txid,
						cold_watermark_versionstamp: [1; 16],
					})?,
				);
				Ok(())
			})
			.await?;

			let recovered = db
				.txn("test_depotburst_mode", move |tx| async move {
					burst_mode::read_branch_signal(&tx, branch_id, Snapshot).await
				})
				.await?;
			assert!(!recovered.active);
			assert_eq!(recovered.lag_txids, 0);
			assert_eq!(recovered.compaction_watermark_txid, head_txid);

			Ok(())
		})
	})
	.await
}
