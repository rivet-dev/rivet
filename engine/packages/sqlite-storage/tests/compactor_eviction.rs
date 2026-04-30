use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		CompactorLease, encode_lease,
		eviction::{EvictableShardVersion, EvictionCompactorConfig, test_hooks},
	},
	constants::{ACCESS_TOUCH_THROTTLE_MS, HOT_CACHE_WINDOW_MS, SHARD_RETENTION_MARGIN},
	keys::{
		CompactorQueueKind, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_hot_pass_txid_key, branch_shard_key, branch_vtx_key,
		branches_bk_pin_key, branches_desc_pin_key, compactor_global_lease_key,
		ctr_eviction_index_key,
	},
	types::ActorBranchId,
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;

fn branch_id(value: u128) -> ActorBranchId {
	ActorBranchId::from_uuid(uuid::Uuid::from_u128(value))
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("sqlite-storage-eviction-compactor-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

async fn seed_eviction_index(
	db: &universaldb::Database,
	rows: Vec<(i64, ActorBranchId)>,
) -> Result<()> {
	db.run(move |tx| {
		let rows = rows.clone();
		async move {
			for (bucket, branch_id) in rows {
				tx.informal()
					.set(&ctr_eviction_index_key(bucket, branch_id), &[]);
			}
			Ok(())
		}
	})
	.await
}

async fn seed_evictable_shard_candidate(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
	shard_id: u32,
	as_of_txid: u64,
	newer_as_of_txid: u64,
) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal().set(
			&branch_manifest_cold_drained_txid_key(branch_id),
			&newer_as_of_txid.to_be_bytes(),
		);
		tx.informal().set(
			&branch_manifest_last_hot_pass_txid_key(branch_id),
			&newer_as_of_txid
				.checked_add(SHARD_RETENTION_MARGIN)
				.and_then(|value| value.checked_add(1))
				.expect("test txid should not overflow")
				.to_be_bytes(),
		);
		tx.informal()
			.set(&branch_shard_key(branch_id, shard_id, as_of_txid), &[1]);
		tx.informal()
			.set(&branch_shard_key(branch_id, shard_id, newer_as_of_txid), &[2]);
		Ok(())
	})
	.await
}

async fn write_branch_pin(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
	pin_key: Vec<u8>,
	versionstamp: [u8; 16],
	txid: u64,
) -> Result<()> {
	db.run(move |tx| {
		let pin_key = pin_key.clone();
		async move {
			tx.informal().set(&pin_key, &versionstamp);
			tx.informal()
				.set(&branch_vtx_key(branch_id, versionstamp), &txid.to_be_bytes());
			Ok(())
		}
	})
	.await
}

#[tokio::test]
async fn sweep_takes_global_lease_and_scans_oldest_candidates_first() -> Result<()> {
	let db = test_db().await?;
	let branch_a = branch_id(0xaaa);
	let branch_b = branch_id(0xbbb);
	let branch_c = branch_id(0xccc);

	seed_eviction_index(&db, vec![(10, branch_a), (2, branch_b), (7, branch_c)]).await?;

	let outcome = test_hooks::sweep_once_for_test(
		&db,
		&EvictionCompactorConfig {
			batch_size: 2,
			..EvictionCompactorConfig::default()
		},
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;

	assert!(outcome.lease_acquired);
	assert_eq!(outcome.scanned_candidates.len(), 2);
	assert_eq!(outcome.scanned_candidates[0].last_access_bucket, 2);
	assert_eq!(outcome.scanned_candidates[0].branch_id, branch_b);
	assert_eq!(outcome.scanned_candidates[1].last_access_bucket, 7);
	assert_eq!(outcome.scanned_candidates[1].branch_id, branch_c);
	assert!(
		read_value(
			&db,
			compactor_global_lease_key(CompactorQueueKind::Eviction)
		)
		.await?
		.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn sweep_skips_when_global_eviction_lease_is_held() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();

	db.run(move |tx| async move {
		tx.informal().set(
			&compactor_global_lease_key(CompactorQueueKind::Eviction),
			&encode_lease(CompactorLease {
				holder_id: holder,
				expires_at_ms: i64::MAX,
			})?,
		);
		Ok(())
	})
	.await?;

	let outcome = test_hooks::sweep_once_for_test(
		&db,
		&EvictionCompactorConfig {
			batch_size: 2,
			..EvictionCompactorConfig::default()
		},
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(
		outcome,
		sqlite_storage::compactor::eviction::EvictionSweepOutcome {
			lease_acquired: false,
			scanned_candidates: Vec::new(),
			evictable_shard_versions: Vec::new(),
		}
	);

	Ok(())
}

#[tokio::test]
async fn predicate_returns_evictable_when_all_gates_pass() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0xddd);
	let now_ms = HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS;

	seed_evictable_shard_candidate(&db, branch_id, 4, 10, 100).await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert_eq!(
		evictable,
		vec![EvictableShardVersion {
			branch_id,
			shard_id: 4,
			as_of_txid: 10,
			last_hot_pass_txid_at_plan: 100 + SHARD_RETENTION_MARGIN + 1,
		}]
	);

	Ok(())
}

#[tokio::test]
async fn predicate_requires_hot_cache_window_to_pass() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0xeee);
	let now_ms = HOT_CACHE_WINDOW_MS - 1;

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert!(evictable.is_empty());

	Ok(())
}

#[tokio::test]
async fn predicate_requires_cold_drained_txid_to_cover_version() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0xfff);
	let now_ms = HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS;

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_manifest_cold_drained_txid_key(branch_id), &9u64.to_be_bytes());
		Ok(())
	})
	.await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert!(evictable.is_empty());

	Ok(())
}

#[tokio::test]
async fn predicate_requires_no_descendant_pin() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1110);
	let now_ms = HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS;
	let mut versionstamp = [0u8; 16];
	versionstamp[15] = 1;

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	write_branch_pin(
		&db,
		branch_id,
		branches_desc_pin_key(branch_id),
		versionstamp,
		10,
	)
	.await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert!(evictable.is_empty());

	Ok(())
}

#[tokio::test]
async fn predicate_requires_no_bookmark_pin() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1111);
	let now_ms = HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS;
	let mut versionstamp = [0u8; 16];
	versionstamp[15] = 2;

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	write_branch_pin(&db, branch_id, branches_bk_pin_key(branch_id), versionstamp, 10).await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert!(evictable.is_empty());

	Ok(())
}

#[tokio::test]
async fn predicate_requires_newer_shard_and_hot_pass_margin() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1112);
	let now_ms = HOT_CACHE_WINDOW_MS + ACCESS_TOUCH_THROTTLE_MS;

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	db.run(move |tx| async move {
		tx.informal().set(
			&branch_manifest_last_hot_pass_txid_key(branch_id),
			&(10 + SHARD_RETENTION_MARGIN - 1).to_be_bytes(),
		);
		tx.informal()
			.set(&branch_shard_key(branch_id, 1, 10), &[3]);
		Ok(())
	})
	.await?;

	let evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, now_ms).await?;

	assert!(evictable.is_empty());

	Ok(())
}
