use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		CompactorLease, encode_lease,
		eviction::{EvictionCompactorConfig, test_hooks},
	},
	keys::{CompactorQueueKind, compactor_global_lease_key, ctr_eviction_index_key},
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
		}
	);

	Ok(())
}
