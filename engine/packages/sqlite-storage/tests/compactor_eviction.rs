use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		CompactorLease, encode_lease,
		eviction::{EvictablePidxEntry, EvictableShardVersion, EvictionCompactorConfig, test_hooks},
		metrics,
	},
	constants::{ACCESS_TOUCH_THROTTLE_MS, HOT_CACHE_WINDOW_MS, SHARD_RETENTION_MARGIN},
	keys::{
		CompactorQueueKind, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_access_bucket_key, branch_manifest_last_access_ts_ms_key,
		branch_manifest_last_hot_pass_txid_key, branch_pidx_key, branch_shard_key, branch_vtx_key,
		branches_bk_pin_key, branches_desc_pin_key, compactor_global_lease_key,
		ctr_eviction_index_key,
	},
	pump::db,
	types::DatabaseBranchId,
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;

fn branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(value))
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

async fn read_i64_le(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<i64>> {
	read_value(db, key).await?.map(|value| {
		Ok(i64::from_le_bytes(
			value
				.as_slice()
				.try_into()
				.expect("test value should be i64"),
		))
	}).transpose()
}

async fn seed_eviction_index(
	db: &universaldb::Database,
	rows: Vec<(i64, DatabaseBranchId)>,
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
	branch_id: DatabaseBranchId,
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
	branch_id: DatabaseBranchId,
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

fn eviction_occ_abort_count() -> u64 {
	let node_id = uuid::Uuid::nil().to_string();
	metrics::SQLITE_EVICTION_OCC_ABORT_TOTAL
		.with_label_values(&[node_id.as_str(), "hot_pass_advanced"])
		.get()
}

#[tokio::test]
async fn access_touch_throttle_bounds_eviction_index_churn_at_1ms_cadence() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1116);

	db.run(move |tx| async move {
		let first_bucket = db::test_hooks::touch_access_if_bucket_advanced_for_test(
			&tx,
			branch_id,
			None,
			1,
		)
		.await?;
		assert_eq!(first_bucket, Some(0));
		Ok(())
	})
	.await?;

	db.run(move |tx| async move {
		for now_ms in 2..ACCESS_TOUCH_THROTTLE_MS {
			let touched = db::test_hooks::touch_access_if_bucket_advanced_for_test(
				&tx,
				branch_id,
				Some(0),
				now_ms,
			)
			.await?;
			assert_eq!(touched, None);
		}
		Ok(())
	})
	.await?;

	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		Some(1)
	);
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
		Some(0)
	);
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(0, branch_id)).await?,
		Some(Vec::new())
	);

	db.run(move |tx| async move {
		let next_bucket = db::test_hooks::touch_access_if_bucket_advanced_for_test(
			&tx,
			branch_id,
			Some(0),
			ACCESS_TOUCH_THROTTLE_MS,
		)
		.await?;
		assert_eq!(next_bucket, Some(1));
		Ok(())
	})
	.await?;

	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_ts_ms_key(branch_id)).await?,
		Some(ACCESS_TOUCH_THROTTLE_MS)
	);
	assert_eq!(
		read_i64_le(&db, branch_manifest_last_access_bucket_key(branch_id)).await?,
		Some(1)
	);
	assert!(read_value(&db, ctr_eviction_index_key(0, branch_id)).await?.is_none());
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(1, branch_id)).await?,
		Some(Vec::new())
	);

	Ok(())
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
			last_access_bucket: 0,
			shard_id: 4,
			as_of_txid: 10,
			last_hot_pass_txid_at_plan: 100 + SHARD_RETENTION_MARGIN + 1,
			shard_value: vec![1],
			pidx_deletes: Vec::new(),
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
async fn sweep_clears_planned_rows_with_compare_and_clear() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1113);
	let old_pidx_key = branch_pidx_key(branch_id, 4);
	let newer_pidx_key = branch_pidx_key(branch_id, 5);

	seed_eviction_index(&db, vec![(0, branch_id)]).await?;
	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	db.run({
		let old_pidx_key = old_pidx_key.clone();
		let newer_pidx_key = newer_pidx_key.clone();
		move |tx| {
			let old_pidx_key = old_pidx_key.clone();
			let newer_pidx_key = newer_pidx_key.clone();
			async move {
				tx.informal().set(&old_pidx_key, &10u64.to_be_bytes());
				tx.informal().set(&newer_pidx_key, &101u64.to_be_bytes());
				Ok(())
			}
		}
	})
	.await?;

	let outcome = test_hooks::sweep_once_for_test(
		&db,
		&EvictionCompactorConfig {
			batch_size: 1,
			..EvictionCompactorConfig::default()
		},
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(outcome.evictable_shard_versions.len(), 1);
	assert!(read_value(&db, branch_shard_key(branch_id, 0, 10)).await?.is_none());
	assert!(read_value(&db, old_pidx_key).await?.is_none());
	assert_eq!(
		read_value(&db, newer_pidx_key).await?,
		Some(101u64.to_be_bytes().to_vec())
	);
	assert_eq!(
		read_value(&db, branch_shard_key(branch_id, 0, 100)).await?,
		Some(vec![2])
	);
	assert_eq!(
		read_value(&db, ctr_eviction_index_key(0, branch_id)).await?,
		Some(Vec::new())
	);

	Ok(())
}

#[tokio::test]
async fn clear_removes_eviction_index_when_last_shard_is_cleared() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1115);

	seed_eviction_index(&db, vec![(0, branch_id)]).await?;
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_shard_key(branch_id, 0, 10), &[1]);
		Ok(())
	})
	.await?;

	let cleared = test_hooks::clear_evictable_shard_versions_for_test(
		&db,
		vec![EvictableShardVersion {
			branch_id,
			last_access_bucket: 0,
			shard_id: 0,
			as_of_txid: 10,
			last_hot_pass_txid_at_plan: 0,
			shard_value: vec![1],
			pidx_deletes: Vec::new(),
		}],
	)
	.await?;

	assert_eq!(cleared.len(), 1);
	assert!(read_value(&db, branch_shard_key(branch_id, 0, 10)).await?.is_none());
	assert!(read_value(&db, ctr_eviction_index_key(0, branch_id)).await?.is_none());

	Ok(())
}

#[tokio::test]
async fn sweep_aborts_clear_when_hot_pass_advances_after_plan() -> Result<()> {
	let db = test_db().await?;
	let branch_id = branch_id(0x1114);
	let before_metric = eviction_occ_abort_count();

	seed_evictable_shard_candidate(&db, branch_id, 0, 10, 100).await?;
	let mut evictable =
		test_hooks::plan_evictable_shard_versions_for_test(&db, branch_id, 0, HOT_CACHE_WINDOW_MS)
			.await?;
	assert_eq!(evictable.len(), 1);
	evictable[0].pidx_deletes.push(EvictablePidxEntry {
		key: branch_pidx_key(branch_id, 1),
		expected_value: 10u64.to_be_bytes().to_vec(),
	});
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_pidx_key(branch_id, 1), &10u64.to_be_bytes());
		tx.informal().set(
			&branch_manifest_last_hot_pass_txid_key(branch_id),
			&(100 + SHARD_RETENTION_MARGIN + 2).to_be_bytes(),
		);
		Ok(())
	})
	.await?;

	let cleared = test_hooks::clear_evictable_shard_versions_for_test(&db, evictable).await?;

	assert!(cleared.is_empty());
	assert_eq!(eviction_occ_abort_count(), before_metric + 1);
	assert_eq!(
		read_value(&db, branch_shard_key(branch_id, 0, 10)).await?,
		Some(vec![1])
	);
	assert_eq!(
		read_value(&db, branch_pidx_key(branch_id, 1)).await?,
		Some(10u64.to_be_bytes().to_vec())
	);

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
