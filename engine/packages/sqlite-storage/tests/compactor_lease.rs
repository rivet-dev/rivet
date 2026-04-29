use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		CompactorLease, RenewOutcome, TakeOutcome, decode_lease, encode_lease, release, renew,
		take,
	},
	keys::meta_compactor_lease_key,
};
use tempfile::Builder;
use tokio::sync::Barrier;
use universaldb::{error::DatabaseError, options::DatabaseOption, utils::IsolationLevel::Snapshot};

const TEST_ACTOR: &str = "lease-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-lease-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn read_lease(db: &universaldb::Database) -> Result<Option<CompactorLease>> {
	db.run(|tx| async move {
		let Some(value) = tx
			.informal()
			.get(&meta_compactor_lease_key(TEST_ACTOR), Snapshot)
			.await?
		else {
			return Ok(None);
		};

		Ok(Some(decode_lease(&value)?))
	})
	.await
}

async fn write_lease(db: &universaldb::Database, lease: CompactorLease) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal()
			.set(&meta_compactor_lease_key(TEST_ACTOR), &encode_lease(lease)?);
		Ok(())
	})
	.await
}

#[tokio::test]
async fn acquire_on_empty_key() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();

	let outcome = db
		.run(move |tx| async move { take(&tx, TEST_ACTOR, holder, 30_000, 1_000).await })
		.await?;

	assert_eq!(outcome, TakeOutcome::Acquired);
	assert_eq!(
		read_lease(&db).await?,
		Some(CompactorLease {
			holder_id: holder,
			expires_at_ms: 31_000,
		})
	);

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn skip_when_another_pod_holds_then_acquire_after_expiry() -> Result<()> {
	let db = test_db().await?;
	let holder_a = NodeId::new();
	let holder_b = NodeId::new();

	db.run(move |tx| async move { take(&tx, TEST_ACTOR, holder_a, 1_000, 0).await })
		.await?;

	let outcome = db
		.run(move |tx| async move { take(&tx, TEST_ACTOR, holder_b, 1_000, 500).await })
		.await?;
	assert_eq!(outcome, TakeOutcome::Skip);
	assert_eq!(read_lease(&db).await?.expect("lease should exist").holder_id, holder_a);

	tokio::time::advance(Duration::from_millis(1_001)).await;

	let outcome = db
		.run(move |tx| async move { take(&tx, TEST_ACTOR, holder_b, 1_000, 1_001).await })
		.await?;
	assert_eq!(outcome, TakeOutcome::Acquired);
	assert_eq!(read_lease(&db).await?.expect("lease should exist").holder_id, holder_b);

	Ok(())
}

#[tokio::test]
async fn racing_takes_leave_one_winner_and_one_occ_abort() -> Result<()> {
	let db = Arc::new(test_db().await?);
	db.set_option(DatabaseOption::TransactionRetryLimit(1))?;

	let barrier = Arc::new(Barrier::new(2));
	let holder_a = NodeId::new();
	let holder_b = NodeId::new();

	let task_a = {
		let db = db.clone();
		let barrier = barrier.clone();
		async move {
			db.run(move |tx| {
				let barrier = barrier.clone();
				async move {
					let outcome = take(&tx, TEST_ACTOR, holder_a, 30_000, 0).await?;
					barrier.wait().await;
					Ok(outcome)
				}
			})
			.await
		}
	};

	let task_b = {
		let db = db.clone();
		let barrier = barrier.clone();
		async move {
			db.run(move |tx| {
				let barrier = barrier.clone();
				async move {
					let outcome = take(&tx, TEST_ACTOR, holder_b, 30_000, 0).await?;
					barrier.wait().await;
					Ok(outcome)
				}
			})
			.await
		}
	};

	let (result_a, result_b) = tokio::join!(task_a, task_b);
	let results = [result_a, result_b];

	assert_eq!(
		results
			.iter()
			.filter(|result| matches!(result, Ok(TakeOutcome::Acquired)))
			.count(),
		1
	);
	assert_eq!(
		results
			.iter()
			.filter(|result| {
				result.as_ref().err().is_some_and(|err| {
					err.chain().any(|cause| {
						cause
							.downcast_ref::<DatabaseError>()
							.is_some_and(|err| matches!(err, DatabaseError::MaxRetriesReached))
					})
				})
			})
			.count(),
		1
	);

	Ok(())
}

#[tokio::test]
async fn renew_success_extends_expiration() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();

	db.run(move |tx| async move { take(&tx, TEST_ACTOR, holder, 1_000, 0).await })
		.await?;

	let outcome = db
		.run(move |tx| async move { renew(&tx, TEST_ACTOR, holder, 2_000, 500).await })
		.await?;

	assert_eq!(outcome, RenewOutcome::Renewed);
	assert_eq!(
		read_lease(&db).await?,
		Some(CompactorLease {
			holder_id: holder,
			expires_at_ms: 2_500,
		})
	);

	Ok(())
}

#[tokio::test]
async fn renew_detects_steal() -> Result<()> {
	let db = test_db().await?;
	let holder_a = NodeId::new();
	let holder_b = NodeId::new();

	write_lease(
		&db,
		CompactorLease {
			holder_id: holder_b,
			expires_at_ms: 30_000,
		},
	)
	.await?;

	let outcome = db
		.run(move |tx| async move { renew(&tx, TEST_ACTOR, holder_a, 30_000, 1_000).await })
		.await?;

	assert_eq!(outcome, RenewOutcome::Stolen);
	assert_eq!(read_lease(&db).await?.expect("lease should exist").holder_id, holder_b);

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn renew_detects_expiry() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();

	db.run(move |tx| async move { take(&tx, TEST_ACTOR, holder, 1_000, 0).await })
		.await?;
	tokio::time::advance(Duration::from_millis(1_001)).await;

	let outcome = db
		.run(move |tx| async move { renew(&tx, TEST_ACTOR, holder, 1_000, 1_001).await })
		.await?;

	assert_eq!(outcome, RenewOutcome::Expired);
	assert_eq!(read_lease(&db).await?.expect("lease should exist").holder_id, holder);

	Ok(())
}

#[tokio::test]
async fn release_clears_key() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();

	db.run(move |tx| async move { take(&tx, TEST_ACTOR, holder, 30_000, 0).await })
		.await?;
	db.run(move |tx| async move { release(&tx, TEST_ACTOR, holder).await })
		.await?;

	assert!(read_lease(&db).await?.is_none());

	Ok(())
}
