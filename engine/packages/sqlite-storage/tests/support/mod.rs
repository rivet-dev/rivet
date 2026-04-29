#![allow(dead_code)]

use std::sync::Arc;

use anyhow::Result;
use namespace::types::SqliteNamespaceConfig;
use rivet_error::RivetError;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{self, AuditFields, ForkDstSpec, ForkMode, OpKind, RestoreMode, RestoreTarget},
	compactor::{self, CheckpointOutcome},
	keys::{
		PAGE_SIZE, checkpoint_meta_key, delta_meta_key, meta_fork_in_progress_key, meta_head_key,
		meta_restore_in_progress_key,
	},
	pump::ActorDb,
	quota,
	types::{
		CheckpointMeta, DBHead, DeltaMeta, DirtyPage, FetchedPage, decode_checkpoint_meta,
		decode_db_head, decode_delta_meta,
	},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};
use uuid::Uuid;

pub async fn test_db(prefix: &str) -> Result<Arc<universaldb::Database>> {
	let path = Builder::new().prefix(prefix).tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
}

pub fn test_ups(name: &str) -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(name.to_string())))
}

pub fn namespace_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		default_retention_ms: 86_400_000,
		default_checkpoint_interval_ms: 1,
		default_max_checkpoints: 25,
		allow_pitr_read: true,
		allow_pitr_destructive: true,
		allow_pitr_admin: true,
		allow_fork: true,
		pitr_max_bytes_per_actor: 100_000_000,
		pitr_namespace_budget_bytes: 100_000_000,
		max_retention_ms: 86_400_000,
		admin_op_rate_per_min: 10,
		concurrent_admin_ops: 4,
		concurrent_forks_per_src: 2,
	}
}

pub fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; PAGE_SIZE as usize],
	}
}

pub fn actor_db(db: Arc<universaldb::Database>, actor_id: &str) -> ActorDb {
	ActorDb::new(
		db,
		test_ups(&format!("restore-{actor_id}")),
		actor_id.to_string(),
		NodeId::new(),
	)
}

pub async fn commit_pages(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	pages: Vec<(u32, u8)>,
	db_size_pages: u32,
	now_ms: i64,
) -> Result<()> {
	actor_db(db, actor_id)
		.commit(
			pages
				.into_iter()
				.map(|(pgno, fill)| page(pgno, fill))
				.collect(),
			db_size_pages,
			now_ms,
		)
		.await
}

pub async fn checkpoint(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	ckp_txid: u64,
) -> Result<CheckpointOutcome> {
	compactor::create_checkpoint(
		db,
		actor_id.to_string(),
		ckp_txid,
		CancellationToken::new(),
		namespace_config(),
	)
	.await
}

pub async fn create_restore_record(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	op_id: Uuid,
) -> Result<()> {
	admin::create_record(
		db,
		op_id,
		OpKind::Restore,
		actor_id.to_string(),
		AuditFields {
			caller_id: "tester".to_string(),
			request_origin_ts_ms: 1_000,
			namespace_id: Uuid::new_v4(),
		},
	)
	.await
}

pub async fn create_fork_record(
	db: Arc<universaldb::Database>,
	src_actor_id: &str,
	op_id: Uuid,
) -> Result<()> {
	admin::create_record(
		db,
		op_id,
		OpKind::Fork,
		src_actor_id.to_string(),
		AuditFields {
			caller_id: "tester".to_string(),
			request_origin_ts_ms: 1_000,
			namespace_id: Uuid::new_v4(),
		},
	)
	.await
}

pub async fn run_restore(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	target: RestoreTarget,
	mode: RestoreMode,
) -> Result<Uuid> {
	let op_id = Uuid::new_v4();
	create_restore_record(Arc::clone(&db), actor_id, op_id).await?;
	compactor::handle_restore(
		db,
		op_id,
		actor_id.to_string(),
		target,
		mode,
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;
	Ok(op_id)
}

pub async fn run_fork(
	db: Arc<universaldb::Database>,
	src_actor_id: &str,
	target: RestoreTarget,
	mode: ForkMode,
	dst: ForkDstSpec,
) -> Result<Uuid> {
	let op_id = Uuid::new_v4();
	create_fork_record(Arc::clone(&db), src_actor_id, op_id).await?;
	compactor::handle_fork(
		db,
		test_ups(&format!("fork-{src_actor_id}")),
		op_id,
		src_actor_id.to_string(),
		target,
		mode,
		dst,
		NodeId::new(),
		CancellationToken::new(),
	)
	.await?;
	Ok(op_id)
}

pub async fn read_pages(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	pgnos: Vec<u32>,
) -> Result<Vec<FetchedPage>> {
	actor_db(db, actor_id).get_pages(pgnos).await
}

pub async fn read_head(db: Arc<universaldb::Database>, actor_id: &str) -> Result<DBHead> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let bytes = tx
				.informal()
				.get(&meta_head_key(&actor_id), Snapshot)
				.await?
				.expect("head should exist");
			decode_db_head(&bytes)
		}
	})
	.await
}

pub async fn read_live_quota(db: Arc<universaldb::Database>, actor_id: &str) -> Result<i64> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move { quota::read_live(&tx, &actor_id).await }
	})
	.await
}

pub async fn marker_exists(db: Arc<universaldb::Database>, actor_id: &str) -> Result<bool> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			Ok(tx
				.informal()
				.get(&meta_restore_in_progress_key(&actor_id), Snapshot)
				.await?
				.is_some())
		}
	})
	.await
}

pub async fn fork_marker_exists(db: Arc<universaldb::Database>, actor_id: &str) -> Result<bool> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			Ok(tx
				.informal()
				.get(&meta_fork_in_progress_key(&actor_id), Snapshot)
				.await?
				.is_some())
		}
	})
	.await
}

pub async fn read_checkpoint_meta(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	ckp_txid: u64,
) -> Result<CheckpointMeta> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let bytes = tx
				.informal()
				.get(&checkpoint_meta_key(&actor_id, ckp_txid), Snapshot)
				.await?
				.expect("checkpoint meta should exist");
			decode_checkpoint_meta(&bytes)
		}
	})
	.await
}

pub async fn read_delta_meta(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	txid: u64,
) -> Result<DeltaMeta> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let bytes = tx
				.informal()
				.get(&delta_meta_key(&actor_id, txid), Snapshot)
				.await?
				.expect("delta meta should exist");
			decode_delta_meta(&bytes)
		}
	})
	.await
}

pub fn assert_actor_restore_error(err: &anyhow::Error) {
	let extracted = RivetError::extract(err);
	assert_eq!(extracted.group(), "sqlite_admin");
	assert_eq!(extracted.code(), "actor_restore_in_progress");
}
