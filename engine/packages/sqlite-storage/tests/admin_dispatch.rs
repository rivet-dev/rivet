use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicUsize, Ordering},
	},
	time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{
		self, AdminOpRecord, AuditFields, OpKind, OpStatus, RestoreMode, RestoreTarget, SqliteOp,
		SqliteOpRequest, SqliteOpSubject, encode_admin_op_record, encode_sqlite_op_request,
	},
	compactor::{
		CompactorConfig, CompactorLease, encode_lease, orphan, worker,
		worker::AdminResumeState,
	},
	keys::{meta_admin_op_key, meta_compactor_lease_key, meta_restore_in_progress_key},
	types::{RestoreMarker, RestoreStep, encode_restore_marker},
};
use tempfile::Builder;
use tokio::sync::{Notify, mpsc};
use universalpubsub::{
	PubSub, PublishOpts,
	driver::{PubSubDriver, SubscriberDriver},
	driver::memory::MemoryDriver,
	pubsub::DriverOutput,
};
use uuid::Uuid;

static HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-admin-dispatch-test".to_string(),
	)))
}

async fn test_db() -> Result<Arc<universaldb::Database>> {
	let path = Builder::new()
		.prefix("sqlite-storage-admin-dispatch-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
}

fn audit() -> AuditFields {
	AuditFields {
		caller_id: "user-1".to_string(),
		request_origin_ts_ms: 1_000,
		namespace_id: Uuid::new_v4(),
	}
}

fn request(op_id: Uuid, actor_id: &str) -> SqliteOpRequest {
	SqliteOpRequest {
		request_id: op_id,
		op: SqliteOp::Restore {
			actor_id: actor_id.to_string(),
			target: RestoreTarget::Txid(7),
			mode: RestoreMode::DryRun,
		},
		audit: audit(),
	}
}

fn test_config(max_concurrent_workers: u32) -> CompactorConfig {
	CompactorConfig {
		max_concurrent_workers,
		..CompactorConfig::default()
	}
}

async fn publish_op(ups: &PubSub, request: SqliteOpRequest) -> Result<()> {
	ups.publish(
		SqliteOpSubject,
		&encode_sqlite_op_request(request)?,
		PublishOpts::one(),
	)
	.await
}

async fn read_record(
	db: Arc<universaldb::Database>,
	op_id: Uuid,
) -> Result<Option<AdminOpRecord>> {
	admin::read(db, op_id).await
}

async fn seed_record(
	db: &universaldb::Database,
	op_id: Uuid,
	actor_id: &str,
	status: OpStatus,
	holder_id: Option<NodeId>,
	created_at_ms: i64,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	let record = AdminOpRecord {
		operation_id: op_id,
		op_kind: OpKind::Restore,
		actor_id: actor_id.clone(),
		created_at_ms,
		last_progress_at_ms: created_at_ms,
		status,
		holder_id,
		progress: None,
		result: None,
		audit: audit(),
	};
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		let record = record.clone();
		async move {
			tx.informal()
				.set(&meta_admin_op_key(&actor_id, op_id), &encode_admin_op_record(record)?);
			Ok(())
		}
	})
	.await
}

async fn seed_restore_marker(
	db: &universaldb::Database,
	actor_id: &str,
	op_id: Uuid,
	holder_id: NodeId,
) -> Result<RestoreMarker> {
	let actor_id = actor_id.to_string();
	let marker = RestoreMarker {
		target_txid: 9,
		ckp_txid: 5,
		started_at_ms: 1_000,
		last_completed_step: RestoreStep::CheckpointCopied,
		holder_id,
		op_id,
	};
	db.run({
		let marker = marker.clone();
		move |tx| {
			let actor_id = actor_id.clone();
			let marker = marker.clone();
			async move {
				tx.informal()
					.set(&meta_restore_in_progress_key(&actor_id), &encode_restore_marker(marker)?);
				Ok(())
			}
		}
	})
	.await?;
	Ok(marker)
}

#[test]
fn module_compiles() {}

#[tokio::test]
async fn op_dispatch_basic() -> Result<()> {
	let _hook_lock = HOOK_LOCK.lock().await;
	let db = test_db().await?;
	let ups = test_ups();
	let holder = NodeId::new();
	let actor_id = "admin-dispatch-basic";
	let op_id = Uuid::new_v4();
	admin::create_record(Arc::clone(&db), op_id, OpKind::Restore, actor_id.to_string(), audit())
		.await?;

	let (tx, mut rx) = mpsc::unbounded_channel();
	let _guard = worker::test_hooks::set_admin_op_hook(move |event| {
		let tx = tx.clone();
		async move {
			tx.send(event).expect("hook receiver should be open");
			Ok(true)
		}
	});
	let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
	worker::test_hooks::set_run_ready_signal(ready_tx);
	let run_handle = tokio::spawn(worker::test_hooks::run_for_test(
		Arc::clone(&db),
		ups.clone(),
		test_config(4),
		holder,
	));
	ready_rx.await.expect("worker should subscribe");

	publish_op(&ups, request(op_id, actor_id)).await?;

	let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
		.await?
		.expect("handler should be invoked");
	assert_eq!(event.request.request_id, op_id);
	assert_eq!(event.record.operation_id, op_id);
	assert_eq!(event.record.status, OpStatus::InProgress);
	assert_eq!(event.record.holder_id, Some(holder));

	let record = read_record(Arc::clone(&db), op_id)
		.await?
		.expect("record should exist");
	assert_eq!(record.status, OpStatus::InProgress);
	assert_eq!(record.holder_id, Some(holder));

	run_handle.abort();
	let _ = run_handle.await;
	Ok(())
}

#[tokio::test]
async fn op_dispatch_concurrent_workers_limit() -> Result<()> {
	let _hook_lock = HOOK_LOCK.lock().await;
	let db = test_db().await?;
	let max_workers = 8;
	let total_ops = 20;
	assert_eq!(CompactorConfig::default().max_concurrent_workers, 64);
	let active = Arc::new(AtomicUsize::new(0));
	let max_seen = Arc::new(AtomicUsize::new(0));
	let started = Arc::new(AtomicUsize::new(0));
	let release = Arc::new(AtomicBool::new(false));
	let release_notify = Arc::new(Notify::new());
	let (started_tx, mut started_rx) = mpsc::unbounded_channel();

	let _guard = worker::test_hooks::set_admin_op_hook({
		let active = Arc::clone(&active);
		let max_seen = Arc::clone(&max_seen);
		let started = Arc::clone(&started);
		let release = Arc::clone(&release);
		let release_notify = Arc::clone(&release_notify);
		move |_event| {
			let active = Arc::clone(&active);
			let max_seen = Arc::clone(&max_seen);
			let started = Arc::clone(&started);
			let release = Arc::clone(&release);
			let release_notify = Arc::clone(&release_notify);
			let started_tx = started_tx.clone();
			async move {
				let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
				max_seen.fetch_max(now_active, Ordering::SeqCst);
				let now_started = started.fetch_add(1, Ordering::SeqCst) + 1;
				started_tx.send(now_started).expect("receiver should be open");
				while !release.load(Ordering::SeqCst) {
					release_notify.notified().await;
				}
				active.fetch_sub(1, Ordering::SeqCst);
				Ok(true)
			}
		}
	});
	let mut requests = Vec::new();
	for idx in 0..total_ops {
		let actor_id = format!("admin-dispatch-concurrent-{idx}");
		let op_id = Uuid::new_v4();
		admin::create_record(
			Arc::clone(&db),
			op_id,
			OpKind::Restore,
			actor_id.clone(),
			audit(),
		)
		.await?;
		requests.push(request(op_id, &actor_id));
	}
	let batch_handle = tokio::spawn(worker::test_hooks::handle_admin_ops_with_limit_for_test(
		Arc::clone(&db),
		requests,
		NodeId::new(),
		max_workers as usize,
	));

	while started.load(Ordering::SeqCst) < max_workers as usize {
		tokio::time::timeout(Duration::from_secs(5), started_rx.recv()).await?;
	}
	tokio::time::sleep(Duration::from_millis(50)).await;
	assert_eq!(active.load(Ordering::SeqCst), max_workers as usize);
	assert_eq!(max_seen.load(Ordering::SeqCst), max_workers as usize);

	release.store(true, Ordering::SeqCst);
	release_notify.notify_waiters();
	while started.load(Ordering::SeqCst) < total_ops as usize {
		tokio::time::timeout(Duration::from_secs(5), started_rx.recv()).await?;
		release_notify.notify_waiters();
	}
	assert!(max_seen.load(Ordering::SeqCst) <= max_workers as usize);

	batch_handle.await??;
	Ok(())
}

#[tokio::test]
async fn orphan_scan_marks_pending_op() -> Result<()> {
	let db = test_db().await?;
	let op_id = Uuid::new_v4();
	seed_record(&db, op_id, "orphan-pending", OpStatus::Pending, None, 1_000).await?;

	let orphaned = orphan::scan_for_orphans(Arc::clone(&db), 32_001).await?;

	assert_eq!(orphaned, 1);
	let record = read_record(db, op_id).await?.expect("record should exist");
	assert_eq!(record.status, OpStatus::Orphaned);
	Ok(())
}

#[tokio::test]
async fn orphan_scan_skips_in_progress() -> Result<()> {
	let db = test_db().await?;
	let op_id = Uuid::new_v4();
	seed_record(
		&db,
		op_id,
		"orphan-in-progress",
		OpStatus::InProgress,
		Some(NodeId::new()),
		1_000,
	)
	.await?;

	let orphaned = orphan::scan_for_orphans(Arc::clone(&db), 32_001).await?;

	assert_eq!(orphaned, 0);
	let record = read_record(db, op_id).await?.expect("record should exist");
	assert_eq!(record.status, OpStatus::InProgress);
	Ok(())
}

#[tokio::test]
async fn resume_on_lease_take_matching_op_id() -> Result<()> {
	let _hook_lock = HOOK_LOCK.lock().await;
	let db = test_db().await?;
	let holder = NodeId::new();
	let actor_id = "admin-resume-matching";
	let op_id = Uuid::new_v4();
	admin::create_record(Arc::clone(&db), op_id, OpKind::Restore, actor_id.to_string(), audit())
		.await?;
	let marker = seed_restore_marker(&db, actor_id, op_id, holder).await?;
	let (tx, mut rx) = mpsc::unbounded_channel();
	let _guard = worker::test_hooks::set_admin_op_hook(move |event| {
		let tx = tx.clone();
		async move {
			tx.send(event.resume).expect("hook receiver should be open");
			Ok(true)
		}
	});

	worker::test_hooks::handle_admin_op_once(Arc::clone(&db), request(op_id, actor_id), holder)
		.await?;

	let resume = rx.recv().await.expect("resume state should be observed");
	assert_eq!(
		resume,
		Some(AdminResumeState::RestoreMatching { marker })
	);
	Ok(())
}

#[tokio::test]
async fn resume_on_lease_take_different_op_id_with_stale_holder() -> Result<()> {
	let _hook_lock = HOOK_LOCK.lock().await;
	let db = test_db().await?;
	let old_holder = NodeId::new();
	let new_holder = NodeId::new();
	let actor_id = "admin-resume-stale-holder";
	let old_op_id = Uuid::new_v4();
	let new_op_id = Uuid::new_v4();
	admin::create_record(
		Arc::clone(&db),
		new_op_id,
		OpKind::Restore,
		actor_id.to_string(),
		audit(),
	)
	.await?;
	let marker = seed_restore_marker(&db, actor_id, old_op_id, old_holder).await?;
	db.run({
		let actor_id = actor_id.to_string();
		move |tx| {
			let actor_id = actor_id.clone();
			async move {
				tx.informal().set(
					&meta_compactor_lease_key(&actor_id),
					&encode_lease(CompactorLease {
						holder_id: old_holder,
						expires_at_ms: 0,
					})?,
				);
				Ok(())
			}
		}
	})
	.await?;
	let (tx, mut rx) = mpsc::unbounded_channel();
	let _guard = worker::test_hooks::set_admin_op_hook(move |event| {
		let tx = tx.clone();
		async move {
			tx.send(event.resume).expect("hook receiver should be open");
			Ok(true)
		}
	});

	worker::test_hooks::handle_admin_op_once(
		Arc::clone(&db),
		request(new_op_id, actor_id),
		new_holder,
	)
	.await?;

	let resume = rx.recv().await.expect("resume state should be observed");
	assert_eq!(
		resume,
		Some(AdminResumeState::RestoreTakeover {
			previous_op_id: old_op_id,
			marker,
		})
	);
	Ok(())
}

#[tokio::test]
async fn unsubscribed_bails() -> Result<()> {
	let _hook_lock = HOOK_LOCK.lock().await;
	let ups = PubSub::new(Arc::new(UnsubscribingDriver));

	let err = worker::test_hooks::unsubscribe_probe_for_test(ups)
	.await
	.expect_err("unsubscribe should make worker return an error");

	assert!(format!("{err:?}").contains("unsubscribed"));
	Ok(())
}

struct UnsubscribingDriver;

#[async_trait]
impl PubSubDriver for UnsubscribingDriver {
	async fn subscribe(&self, _subject: &str) -> Result<Box<dyn SubscriberDriver>> {
		Ok(Box::new(UnsubscribingSubscriber))
	}

	async fn queue_subscribe(
		&self,
		_subject: &str,
		_queue: &str,
	) -> Result<Box<dyn SubscriberDriver>> {
		Ok(Box::new(UnsubscribingSubscriber))
	}

	async fn publish(&self, _subject: &str, _message: &[u8]) -> Result<()> {
		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		Ok(())
	}

	fn max_message_size(&self) -> usize {
		1024
	}
}

struct UnsubscribingSubscriber;

#[async_trait]
impl SubscriberDriver for UnsubscribingSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		Ok(DriverOutput::Unsubscribed)
	}
}
