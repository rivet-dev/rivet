mod fault_common;

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlite_storage::{
	compactor::eviction::test_hooks,
	constants::{ACCESS_TOUCH_THROTTLE_MS, HOT_CACHE_WINDOW_MS, SHARD_RETENTION_MARGIN},
	keys::{
		branch_commit_key, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_hot_pass_txid_key, branch_shard_key,
	},
	pump::branch,
	types::{ActorBranchId, NamespaceBranchId, decode_commit_row},
};

#[tokio::test]
async fn concurrent_fork_during_eviction() -> Result<()> {
	let db = Arc::new(fault_common::test_db("sqlite-storage-fork-during-evict-").await?);
	let actor_db = fault_common::actor_db(Arc::clone(&db), fault_common::TEST_ACTOR);
	actor_db.commit(vec![fault_common::page(1, 0x11)], 2, 1_000).await?;
	let branch_id = fault_common::actor_branch_id_for(&db, fault_common::TEST_ACTOR).await?;
	let commit_bytes = fault_common::read_value(&db, branch_commit_key(branch_id, 1))
		.await?
		.context("commit row should exist")?;
	let commit = decode_commit_row(&commit_bytes)?;

	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_manifest_cold_drained_txid_key(branch_id), &100u64.to_be_bytes());
		tx.informal().set(
			&branch_manifest_last_hot_pass_txid_key(branch_id),
			&(100 + SHARD_RETENTION_MARGIN + 1).to_be_bytes(),
		);
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 1), b"fork-source");
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 100), b"newer");
		Ok(())
	})
	.await?;

	let planned = test_hooks::plan_evictable_shard_versions_for_test(
		&db,
		branch_id,
		0,
		HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS,
	)
	.await?;
	assert_eq!(planned.len(), 1);

	let forked_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0xfeed));
	db.run(move |tx| async move {
		branch::derive_branch_at(
			&tx,
			branch_id,
			commit.versionstamp,
			forked_branch_id,
			NamespaceBranchId::nil(),
			None,
		)
		.await
	})
	.await?;

	let cleared = test_hooks::clear_evictable_shard_versions_for_test(&db, planned).await?;
	assert!(
		cleared.is_empty(),
		"eviction clear should no-op after fork writes desc_pin"
	);
	assert_eq!(
		fault_common::read_value(&db, branch_shard_key(branch_id, 0, 1)).await?,
		Some(b"fork-source".to_vec())
	);

	Ok(())
}
