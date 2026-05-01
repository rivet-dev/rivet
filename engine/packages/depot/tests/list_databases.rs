use std::{collections::BTreeSet, sync::Arc};

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use depot::{
	keys::{database_pointer_cur_key, branch_commit_key, branch_meta_head_key, branches_refcount_key, namespace_pointer_cur_key},
	conveyer::{Db, branch},
	types::{
		DatabaseBranchId, CommitRow, DBHead, DirtyPage, NamespaceBranchId, NamespaceId,
		ResolvedVersionstamp, decode_database_pointer, decode_commit_row, decode_db_head,
		decode_namespace_pointer,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const FIRST_ACTOR: &str = "first-database";
const SECOND_ACTOR: &str = "second-database";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("depot-list-databases-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"depot-list-databases-test".to_string(),
	)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
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

async fn read_namespace_branch_id(
	db: &universaldb::Database,
	namespace_id: NamespaceId,
) -> Result<NamespaceBranchId> {
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");

	Ok(decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch)
}

async fn read_database_branch_id(
	db: &universaldb::Database,
	namespace_id: NamespaceId,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let namespace_branch = read_namespace_branch_id(db, namespace_id).await?;
	let bytes = read_value(db, database_pointer_cur_key(namespace_branch, database_id))
		.await?
		.expect("database pointer should exist");

	Ok(decode_database_pointer(&bytes)?.current_branch)
}

async fn read_head_commit(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<CommitRow> {
	let head_bytes = read_value(db, branch_meta_head_key(branch_id))
		.await?
		.expect("branch head should exist");
	let head: DBHead = decode_db_head(&head_bytes)?;
	let commit_bytes = read_value(db, branch_commit_key(branch_id, head.head_txid))
		.await?
		.expect("head commit row should exist");

	decode_commit_row(&commit_bytes)
}

async fn read_refcount(db: &universaldb::Database, branch_id: DatabaseBranchId) -> Result<i64> {
	let bytes = read_value(db, branches_refcount_key(branch_id))
		.await?
		.expect("branch refcount should exist");
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.expect("branch refcount should be i64 LE");

	Ok(i64::from_le_bytes(bytes))
}

#[tokio::test]
async fn delete_database_in_forked_namespace_hides_in_child_only() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace = NamespaceId::from_gas_id(test_namespace());
	let database_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		FIRST_ACTOR.to_string(),
		NodeId::new(),
	);
	database_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
	let database_id = read_database_branch_id(&db, namespace, FIRST_ACTOR).await?;
	let commit = read_head_commit(&db, database_id).await?;
	let forked_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		namespace,
		ResolvedVersionstamp {
			versionstamp: commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	let parent_databases = branch::list_databases(&db, namespace).await?;
	let child_databases = branch::list_databases(&db, forked_namespace).await?;
	assert_eq!(parent_databases, vec![database_id]);
	assert_eq!(child_databases, vec![database_id]);

	branch::delete_database(&db, forked_namespace, database_id).await?;

	assert_eq!(branch::list_databases(&db, forked_namespace).await?, Vec::new());
	assert_eq!(branch::list_databases(&db, namespace).await?, vec![database_id]);
	assert_eq!(read_refcount(&db, database_id).await?, 0);

	Ok(())
}

#[tokio::test]
async fn fork_namespace_filters_source_databases_created_after_fork() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace = NamespaceId::from_gas_id(test_namespace());
	let first_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		FIRST_ACTOR.to_string(),
		NodeId::new(),
	);
	first_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
	let first_database_id = read_database_branch_id(&db, namespace, FIRST_ACTOR).await?;
	let first_commit = read_head_commit(&db, first_database_id).await?;
	let forked_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		namespace,
		ResolvedVersionstamp {
			versionstamp: first_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	let second_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		SECOND_ACTOR.to_string(),
		NodeId::new(),
	);
	second_db.commit(vec![page(1, 0x22)], 1, 2_000).await?;
	let second_database_id = read_database_branch_id(&db, namespace, SECOND_ACTOR).await?;

	assert_eq!(
		branch::list_databases(&db, namespace)
			.await?
			.into_iter()
			.collect::<BTreeSet<_>>(),
		BTreeSet::from([first_database_id, second_database_id])
	);
	assert_eq!(
		branch::list_databases(&db, forked_namespace).await?,
		vec![first_database_id]
	);

	Ok(())
}

#[tokio::test]
async fn parent_tombstone_visibility_is_capped_across_deep_namespace_chain() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let namespace = NamespaceId::from_gas_id(test_namespace());
	let first_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		FIRST_ACTOR.to_string(),
		NodeId::new(),
	);
	first_db.commit(vec![page(1, 0x11)], 1, 1_000).await?;
	let first_database_id = read_database_branch_id(&db, namespace, FIRST_ACTOR).await?;
	let first_commit = read_head_commit(&db, first_database_id).await?;

	let before_delete_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		namespace,
		ResolvedVersionstamp {
			versionstamp: first_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	branch::delete_database(&db, namespace, first_database_id).await?;

	let second_db = Db::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		SECOND_ACTOR.to_string(),
		NodeId::new(),
	);
	second_db.commit(vec![page(1, 0x22)], 1, 2_000).await?;
	let second_database_id = read_database_branch_id(&db, namespace, SECOND_ACTOR).await?;
	let second_commit = read_head_commit(&db, second_database_id).await?;

	let after_delete_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		namespace,
		ResolvedVersionstamp {
			versionstamp: second_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;
	let deep_after_delete_namespace = branch::fork_namespace(
		&db,
		&test_ups(),
		after_delete_namespace,
		ResolvedVersionstamp {
			versionstamp: second_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	assert_eq!(
		branch::list_databases(&db, before_delete_namespace).await?,
		vec![first_database_id]
	);
	assert_eq!(branch::list_databases(&db, namespace).await?, vec![second_database_id]);
	assert_eq!(
		branch::list_databases(&db, after_delete_namespace).await?,
		vec![second_database_id]
	);
	assert_eq!(
		branch::list_databases(&db, deep_after_delete_namespace).await?,
		vec![second_database_id]
	);

	Ok(())
}
