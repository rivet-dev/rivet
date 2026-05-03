mod common;

use anyhow::Result;
use depot::{
	HOT_BURST_COLD_LAG_THRESHOLD_TXIDS, burst_mode,
	keys::{
		branch_compaction_root_key, branch_manifest_cold_drained_txid_key, branch_meta_head_key,
	},
	types::{CompactionRoot, DBHead, DatabaseBranchId, encode_compaction_root, encode_db_head},
};
use universaldb::utils::IsolationLevel::Snapshot;

fn head_with_branch(branch_id: DatabaseBranchId, head_txid: u64) -> DBHead {
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
async fn burst_signal_is_derived_from_workflow_compaction_root() -> Result<()> {
	common::test_matrix("depot-burst-mode", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = DatabaseBranchId::new_v4();
			db.run(move |tx| async move {
				tx.informal().set(
					&branch_meta_head_key(branch_id),
					&encode_db_head(head_with_branch(
						branch_id,
						HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
					))?,
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
				tx.informal().set(
					&branch_manifest_cold_drained_txid_key(branch_id),
					&HOT_BURST_COLD_LAG_THRESHOLD_TXIDS.to_be_bytes(),
				);
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
					&branch_compaction_root_key(branch_id),
					&encode_compaction_root(CompactionRoot {
						schema_version: 1,
						manifest_generation: 2,
						hot_watermark_txid: HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
						cold_watermark_txid: HOT_BURST_COLD_LAG_THRESHOLD_TXIDS,
						cold_watermark_versionstamp: [1; 16],
					})?,
				);
				tx.informal().set(
					&branch_manifest_cold_drained_txid_key(branch_id),
					&0_u64.to_be_bytes(),
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
		})
	})
	.await
}
