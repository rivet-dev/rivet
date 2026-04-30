use std::sync::Arc;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{
		SQLITE_COLD_COMPACT_PAYLOAD_VERSION, SQLITE_COLD_COMPACT_SUBJECT,
		SqliteColdCompactPayload, SqliteColdCompactSubject,
		cold::{
			ColdCompactorConfig, ColdCompactorLease, ColdRenewOutcome, ColdTakeOutcome,
			decode_cold_lease, encode_cold_lease, release, renew, take, worker,
		},
		decode_cold_compact_payload, encode_cold_compact_payload,
	},
	keys::branch_meta_cold_lease_key,
	types::{ActorBranchId, BookmarkStr},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;

fn actor_branch_id() -> ActorBranchId {
	ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x1234_5678_9abc_def0_0123_4567_89ab_cdef))
}

fn bookmark() -> BookmarkStr {
	BookmarkStr::new("0000018bcfe56800-0000000000000007").expect("bookmark should be valid")
}

fn payload() -> SqliteColdCompactPayload {
	SqliteColdCompactPayload::CreatePinnedBookmark {
		actor_id: "actor-a".to_string(),
		actor_branch_id: actor_branch_id(),
		bookmark: bookmark(),
		versionstamp: [7; 16],
	}
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-cold-compactor-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn read_lease(db: &universaldb::Database) -> Result<Option<ColdCompactorLease>> {
	db.run(|tx| async move {
		let Some(value) = tx
			.informal()
			.get(&branch_meta_cold_lease_key(actor_branch_id()), Snapshot)
			.await?
		else {
			return Ok(None);
		};

		Ok(Some(decode_cold_lease(&value)?))
	})
	.await
}

async fn write_lease(db: &universaldb::Database, lease: ColdCompactorLease) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_meta_cold_lease_key(actor_branch_id()), &encode_cold_lease(lease)?);
		Ok(())
	})
	.await
}

#[test]
fn cold_subject_uses_constant_subject_string() {
	assert_eq!(SqliteColdCompactSubject.to_string(), SQLITE_COLD_COMPACT_SUBJECT);
	assert_eq!(SQLITE_COLD_COMPACT_SUBJECT, "sqlite.cold_compact");
}

#[test]
fn cold_payload_round_trips_with_embedded_version() {
	let encoded = encode_cold_compact_payload(payload()).expect("payload should encode");
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_COLD_COMPACT_PAYLOAD_VERSION
	);

	let decoded = decode_cold_compact_payload(&encoded).expect("payload should decode");
	assert_eq!(decoded, payload());
}

#[tokio::test]
async fn cold_lease_acquire_renew_and_release() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();
	let branch_id = actor_branch_id();

	let outcome = db
		.run(move |tx| async move { take(&tx, branch_id, holder, 30_000, 1_000).await })
		.await?;
	assert_eq!(outcome, ColdTakeOutcome::Acquired);
	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: 31_000,
		})
	);

	let outcome = db
		.run(move |tx| async move { renew(&tx, branch_id, holder, 40_000, 2_000).await })
		.await?;
	assert_eq!(outcome, ColdRenewOutcome::Renewed);
	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: 42_000,
		})
	);

	db.run(move |tx| async move { release(&tx, branch_id, holder).await })
		.await?;
	assert_eq!(read_lease(&db).await?, None);

	Ok(())
}

#[tokio::test]
async fn cold_worker_handles_payload_and_releases_lease() -> Result<()> {
	let db = Arc::new(test_db().await?);

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig {
			lease_ttl_ms: 200,
			lease_renew_interval_ms: 40,
			lease_margin_ms: 80,
			cold_compact_delta_threshold: 1024,
			max_concurrent_workers: 4,
			ups_subject: SQLITE_COLD_COMPACT_SUBJECT.to_string(),
		},
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(read_lease(&db).await?, None);

	Ok(())
}

#[tokio::test]
async fn cold_worker_skips_when_another_holder_has_lease() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let holder = NodeId::new();
	write_lease(
		&db,
		ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: i64::MAX,
		},
	)
	.await?;

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: i64::MAX,
		})
	);

	Ok(())
}
