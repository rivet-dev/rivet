use std::{sync::Arc, time::Duration};

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::compactor::{
	CompactorConfig, CompactorLease, SQLITE_COMPACT_PAYLOAD_VERSION, SQLITE_COMPACT_SUBJECT,
	SqliteCompactPayload, SqliteCompactSubject, compact::test_hooks, decode_compact_payload,
	encode_compact_payload, encode_lease, publish_compact_trigger, worker,
};
use sqlite_storage::{
	keys::{PAGE_SIZE, delta_chunk_key, meta_compact_key, meta_compactor_lease_key, meta_head_key, pidx_delta_key},
	ltx::{LtxHeader, encode_ltx_v3},
	types::{DBHead, DirtyPage, MetaCompact, encode_db_head, encode_meta_compact},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};
use universaldb::utils::IsolationLevel::Snapshot;

static PAUSE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-compactor-dispatch-test".to_string(),
	)))
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-dispatch-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

fn encoded_blob(txid: u64, pages: &[(u32, u8)]) -> Result<Vec<u8>> {
	let pages = pages
		.iter()
		.map(|(pgno, fill)| page(*pgno, *fill))
		.collect::<Vec<_>>();

	encode_ltx_v3(LtxHeader::delta(txid, 128, 999), &pages)
}

async fn seed_compaction_case(db: &universaldb::Database, actor_id: &str) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&meta_head_key(&actor_id),
				&encode_db_head(DBHead {
					head_txid: 1,
					db_size_pages: 128,
					#[cfg(debug_assertions)]
					generation: 0,
				})?,
			);
			tx.informal().set(
				&meta_compact_key(&actor_id),
				&encode_meta_compact(MetaCompact {
					materialized_txid: 0,
				})?,
			);
			tx.informal()
				.set(&delta_chunk_key(&actor_id, 1, 0), &encoded_blob(1, &[(1, 0x11)])?);
			tx.informal()
				.set(&pidx_delta_key(&actor_id, 1), &1_u64.to_be_bytes());
			Ok(())
		}
	})
	.await
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

async fn steal_lease(db: &universaldb::Database, actor_id: &str) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&meta_compactor_lease_key(&actor_id),
				&encode_lease(CompactorLease {
					holder_id: NodeId::new(),
					expires_at_ms: i64::MAX,
				})?,
			);
			Ok(())
		}
	})
	.await
}

fn fast_config() -> CompactorConfig {
	CompactorConfig {
		lease_ttl_ms: 200,
		lease_renew_interval_ms: 40,
		lease_margin_ms: 80,
		compaction_delta_threshold: 1,
		batch_size_deltas: 32,
		max_concurrent_workers: 4,
		ups_subject: SQLITE_COMPACT_SUBJECT.to_string(),
		#[cfg(debug_assertions)]
		quota_validate_every: 16,
	}
}

#[test]
fn module_compiles() {}

#[test]
fn compact_subject_uses_constant_subject_string() {
	assert_eq!(SqliteCompactSubject.to_string(), SQLITE_COMPACT_SUBJECT);
	assert_eq!(SQLITE_COMPACT_SUBJECT, "sqlite.compact");
}

#[test]
fn compact_payload_round_trips_with_embedded_version() {
	for payload in [
		SqliteCompactPayload {
			actor_id: String::new(),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
		SqliteCompactPayload {
			actor_id: "actor-a".to_string(),
			commit_bytes_since_rollup: u64::MAX,
			read_bytes_since_rollup: u64::MAX - 1,
		},
	] {
		let encoded = encode_compact_payload(payload.clone()).expect("payload should encode");
		assert_eq!(
			u16::from_le_bytes([encoded[0], encoded[1]]),
			SQLITE_COMPACT_PAYLOAD_VERSION
		);

		let decoded = decode_compact_payload(&encoded).expect("payload should decode");
		assert_eq!(decoded, payload);
	}
}

#[tokio::test]
async fn publish_compact_trigger_returns_unit_not_future() {
	let ups = test_ups();
	let _: () = publish_compact_trigger(&ups, "actor-1");
}

#[tokio::test(start_paused = true)]
async fn publish_compact_trigger_does_not_block_caller() {
	let ups = test_ups();
	let now = tokio::time::Instant::now();

	let _: () = publish_compact_trigger(&ups, "actor-1");

	assert_eq!(tokio::time::Instant::now(), now);
}

#[tokio::test]
async fn publish_compact_trigger_sends_fire_and_forget_ups_message() {
	let ups = test_ups();
	let mut sub = ups
		.queue_subscribe(SqliteCompactSubject, "compactor")
		.await
		.expect("subscriber should start");

	publish_compact_trigger(&ups, "actor-a");

	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("trigger should publish")
		.expect("subscriber should receive");

	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_compact_payload(&msg.payload).expect("payload should decode");

	assert_eq!(
		payload,
		SqliteCompactPayload {
			actor_id: "actor-a".to_string(),
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		}
	);
}

#[tokio::test]
async fn ups_trigger_arrives_and_spawned_handler_compacts() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let ups = test_ups();
	let actor_id = "actor-worker-basic";
	seed_compaction_case(&db, actor_id).await?;
	let mut sub = ups
		.queue_subscribe(SqliteCompactSubject, "compactor")
		.await
		.expect("subscriber should start");

	publish_compact_trigger(&ups, actor_id);

	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await
		.expect("trigger should publish")
		.expect("subscriber should receive");
	let NextOutput::Message(msg) = msg else {
		panic!("subscriber unexpectedly unsubscribed");
	};
	let payload = decode_compact_payload(&msg.payload).expect("payload should decode");
	let handle = tokio::spawn(worker::test_hooks::handle_trigger_once(
		Arc::clone(&db),
		payload.actor_id,
		fast_config(),
		CancellationToken::new(),
	));

	handle.await.expect("handler should not panic")?;

	assert!(
		read_value(&db, delta_chunk_key(actor_id, 1, 0))
			.await?
			.is_none()
	);
	Ok(())
}

#[tokio::test]
async fn lease_renewal_extends_local_deadline_mid_flight() -> Result<()> {
	let _pause_test_lock = PAUSE_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let actor_id = "actor-worker-renew";
	seed_compaction_case(&db, actor_id).await?;
	let (_guard, reached, release) = test_hooks::pause_after_plan(actor_id);
	let handle = tokio::spawn(worker::test_hooks::handle_trigger_once(
		Arc::clone(&db),
		actor_id.to_string(),
		fast_config(),
		CancellationToken::new(),
	));

	reached.notified().await;
	tokio::task::yield_now().await;
	tokio::time::sleep(Duration::from_millis(250)).await;
	tokio::task::yield_now().await;

	assert!(!handle.is_finished());
	release.notify_waiters();
	handle.await.expect("handler should not panic")?;

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn lease_renewal_failure_cancels_compaction() -> Result<()> {
	let _pause_test_lock = PAUSE_TEST_LOCK.lock().await;
	let db = Arc::new(test_db().await?);
	let actor_id = "actor-worker-stolen";
	seed_compaction_case(&db, actor_id).await?;
	let (_guard, reached, release) = test_hooks::pause_after_plan(actor_id);
	let handle = tokio::spawn(worker::test_hooks::handle_trigger_once(
		Arc::clone(&db),
		actor_id.to_string(),
		fast_config(),
		CancellationToken::new(),
	));

	reached.notified().await;
	tokio::task::yield_now().await;
	steal_lease(&db, actor_id).await?;
	tokio::time::advance(Duration::from_millis(80)).await;
	tokio::task::yield_now().await;
	release.notify_waiters();

	let err = handle
		.await
		.expect("handler should not panic")
		.expect_err("stolen lease should cancel compaction");
	assert!(err.to_string().contains("sqlite compaction cancelled"));

	Ok(())
}
