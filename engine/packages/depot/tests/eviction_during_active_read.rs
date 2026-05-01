mod fault_common;

use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use depot::{
	compactor::eviction::{EvictionCompactorConfig, test_hooks},
	keys::{
		branch_manifest_cold_drained_txid_key, branch_manifest_last_access_bucket_key,
		branch_manifest_last_hot_pass_txid_key, branch_shard_key, ctr_eviction_index_key,
	},
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn eviction_during_active_read() -> Result<()> {
	let db = Arc::new(fault_common::test_db("depot-evict-active-read-").await?);
	let database_db = fault_common::make_db(Arc::clone(&db), fault_common::TEST_DATABASE);
	database_db.commit(vec![fault_common::page(1, 0x11)], 2, 1_000).await?;
	let branch_id = fault_common::database_branch_id_for(&db, fault_common::TEST_DATABASE).await?;

	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_manifest_last_access_bucket_key(branch_id), &0_i64.to_le_bytes());
		tx.informal()
			.set(&ctr_eviction_index_key(0, branch_id), &[]);
		tx.informal()
			.set(&branch_manifest_cold_drained_txid_key(branch_id), &100u64.to_be_bytes());
		tx.informal()
			.set(&branch_manifest_last_hot_pass_txid_key(branch_id), &200u64.to_be_bytes());
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 10), b"old-shard");
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 100), b"new-shard");
		Ok(())
	})
	.await?;

	let pages = database_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(vec![0x11; depot::keys::PAGE_SIZE as usize]));
	assert!(
		fault_common::read_value(&db, ctr_eviction_index_key(0, branch_id))
			.await?
			.is_none(),
		"active read should re-key the eviction index out of the stale bucket"
	);

	let outcome = test_hooks::sweep_once_for_test(
		&db,
		&EvictionCompactorConfig::default(),
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;
	assert!(outcome.evictable_shard_versions.is_empty());
	assert_eq!(
		fault_common::read_value(&db, branch_shard_key(branch_id, 0, 10)).await?,
		Some(b"old-shard".to_vec())
	);

	Ok(())
}
