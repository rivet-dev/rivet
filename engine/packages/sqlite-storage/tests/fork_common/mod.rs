#![allow(dead_code)]

use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	keys::{
		actor_pointer_cur_key, branch_commit_key, branch_meta_head_key, branches_list_key,
		namespace_branches_list_key, namespace_pointer_cur_key,
	},
	pump::ActorDb,
	types::{
		ActorBranchId, ActorBranchRecord, CommitRow, DirtyPage, NamespaceBranchId,
		NamespaceBranchRecord, NamespaceId, decode_actor_branch_record, decode_actor_pointer,
		decode_commit_row, decode_db_head, decode_namespace_branch_record, decode_namespace_pointer,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

pub const TEST_ACTOR: &str = "fork-source";

pub fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

pub fn target_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

pub async fn test_db() -> Result<Arc<universaldb::Database>> {
	let path = Builder::new().prefix("sqlite-storage-fork-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
}

pub fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-fork-test".to_string(),
	)))
}

pub fn actor_db(
	db: Arc<universaldb::Database>,
	namespace_id: Id,
	actor_id: impl Into<String>,
) -> ActorDb {
	ActorDb::new(db, test_ups(), namespace_id, actor_id.into(), NodeId::new())
}

pub fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; sqlite_storage::keys::PAGE_SIZE as usize],
	}
}

pub fn page_bytes(fill: u8) -> Vec<u8> {
	vec![fill; sqlite_storage::keys::PAGE_SIZE as usize]
}

pub async fn read_value(
	db: &universaldb::Database,
	key: Vec<u8>,
) -> Result<Option<Vec<u8>>> {
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

pub async fn read_namespace_branch_id_for(
	db: &universaldb::Database,
	namespace_id: Id,
) -> Result<NamespaceBranchId> {
	let namespace_id = NamespaceId::from_gas_id(namespace_id);
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");

	Ok(decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch)
}

pub async fn read_actor_branch_id(
	db: &universaldb::Database,
	namespace_id: Id,
	actor_id: &str,
) -> Result<ActorBranchId> {
	let namespace_branch = read_namespace_branch_id_for(db, namespace_id).await?;
	let bytes = read_value(db, actor_pointer_cur_key(namespace_branch, actor_id))
		.await?
		.expect("actor pointer should exist");

	Ok(decode_actor_pointer(&bytes)?.current_branch)
}

pub async fn read_actor_branch_record(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
) -> Result<ActorBranchRecord> {
	let bytes = read_value(db, branches_list_key(branch_id))
		.await?
		.expect("actor branch record should exist");

	decode_actor_branch_record(&bytes)
}

pub async fn read_namespace_branch_record(
	db: &universaldb::Database,
	branch_id: NamespaceBranchId,
) -> Result<NamespaceBranchRecord> {
	let bytes = read_value(db, namespace_branches_list_key(branch_id))
		.await?
		.expect("namespace branch record should exist");

	decode_namespace_branch_record(&bytes)
}

pub async fn read_head_commit(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
) -> Result<CommitRow> {
	let head_bytes = read_value(db, branch_meta_head_key(branch_id))
		.await?
		.expect("branch head should exist");
	let head = decode_db_head(&head_bytes)?;
	let bytes = read_value(db, branch_commit_key(branch_id, head.head_txid))
		.await?
		.expect("branch commit row should exist");

	decode_commit_row(&bytes)
}

pub async fn read_commit(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<CommitRow> {
	let bytes = read_value(db, branch_commit_key(branch_id, txid))
		.await?
		.expect("branch commit row should exist");

	decode_commit_row(&bytes)
}

pub fn assert_storage_error(
	err: &anyhow::Error,
	expected: sqlite_storage::error::SqliteStorageError,
) {
	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| err == &expected),
		"expected {expected:?}, got {err:?}",
	);
}
