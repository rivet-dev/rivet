use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	keys::{
		actor_pointer_cur_key, branch_commit_key, branch_meta_head_at_fork_key, branches_bk_pin_key,
		branches_desc_pin_key, branches_list_key, branches_refcount_key,
		namespace_branches_bk_pin_key, namespace_branches_desc_pin_key,
		namespace_branches_list_key, namespace_branches_refcount_key,
		namespace_branches_tier_state_key, namespace_pointer_cur_key,
	},
	pump::{ActorDb, branch},
	types::{
		ActorBranchId, ActorBranchRecord, BranchState, DBHead, DirtyPage, NamespaceBranchId,
		NamespaceBranchRecord, NamespaceId, decode_actor_branch_record, decode_actor_pointer,
		decode_commit_row, decode_db_head, decode_namespace_branch_record, decode_namespace_pointer,
		encode_actor_branch_record, encode_namespace_branch_record,
	},
};
use tempfile::Builder;
use universaldb::{options::MutationType, utils::IsolationLevel::Snapshot};
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const TEST_ACTOR: &str = "test-actor";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-branch-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-pump-branch-test".to_string(),
	)))
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; sqlite_storage::keys::PAGE_SIZE as usize],
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

async fn read_namespace_branch_id(db: &universaldb::Database) -> Result<NamespaceBranchId> {
	let namespace_id = NamespaceId::from_gas_id(test_namespace());
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");

	Ok(decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch)
}

async fn read_branch_id(db: &universaldb::Database) -> Result<ActorBranchId> {
	let namespace_branch = read_namespace_branch_id(db).await?;
	let bytes = read_value(db, actor_pointer_cur_key(namespace_branch, TEST_ACTOR))
		.await?
		.expect("actor pointer should exist");

	Ok(decode_actor_pointer(&bytes)?.current_branch)
}

async fn read_refcount(db: &universaldb::Database, branch_id: ActorBranchId) -> Result<i64> {
	let bytes = read_value(db, branches_refcount_key(branch_id))
		.await?
		.expect("branch refcount should exist");
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.expect("branch refcount should be i64 LE");

	Ok(i64::from_le_bytes(bytes))
}

async fn read_namespace_refcount(
	db: &universaldb::Database,
	branch_id: NamespaceBranchId,
) -> Result<i64> {
	let bytes = read_value(db, namespace_branches_refcount_key(branch_id))
		.await?
		.expect("namespace branch refcount should exist");
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.expect("namespace branch refcount should be i64 LE");

	Ok(i64::from_le_bytes(bytes))
}

#[tokio::test]
async fn derive_branch_at_snapshots_head_and_writes_branch_metadata() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	actor_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;
	let source_branch_id = read_branch_id(&db).await?;
	let namespace_branch_id = read_namespace_branch_id(&db).await?;
	let first_commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
		.await?
		.expect("first commit row should exist");
	let first_commit = decode_commit_row(&first_commit_bytes)?;
	let new_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x9999));

	db.run(move |tx| async move {
		branch::derive_branch_at(
			&tx,
			source_branch_id,
			first_commit.versionstamp,
			new_branch_id,
			namespace_branch_id,
			None,
		)
		.await
	})
	.await?;

	let head_bytes = read_value(&db, branch_meta_head_at_fork_key(new_branch_id))
		.await?
		.expect("head_at_fork should exist");
	assert_eq!(
		decode_db_head(&head_bytes)?,
		DBHead {
			head_txid: 1,
			db_size_pages: 2,
			post_apply_checksum: first_commit.post_apply_checksum,
			branch_id: new_branch_id,
			#[cfg(debug_assertions)]
			generation: 0,
		}
	);
	let record_bytes = read_value(&db, branches_list_key(new_branch_id))
		.await?
		.expect("derived branch record should exist");
	let record = decode_actor_branch_record(&record_bytes)?;
	assert_eq!(record.branch_id, new_branch_id);
	assert_eq!(record.namespace_branch, namespace_branch_id);
	assert_eq!(record.parent, Some(source_branch_id));
	assert_eq!(record.parent_versionstamp, Some(first_commit.versionstamp));
	assert_eq!(record.root_versionstamp, first_commit.versionstamp);
	assert_eq!(record.fork_depth, 1);
	assert_eq!(record.created_from_bookmark, None);
	assert_eq!(record.state, BranchState::Live);
	assert_eq!(read_refcount(&db, source_branch_id).await?, 2);
	assert_eq!(read_refcount(&db, new_branch_id).await?, 1);
	assert_eq!(
		read_value(&db, branches_desc_pin_key(source_branch_id)).await?,
		Some(first_commit.versionstamp.to_vec())
	);

	Ok(())
}

#[tokio::test]
async fn derive_branch_at_rejects_expired_pin_before_copying_head() -> Result<()> {
	let db = test_db().await?;
	let source_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x1111));
	let new_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x2222));
	let namespace_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0x3333));
	let at_versionstamp = [1; 16];
	let pin_versionstamp = [2; 16];

	seed_branch_record(&db, source_branch_id, namespace_branch_id, 0).await?;
	db.run(move |tx| async move {
		tx.informal().atomic_op(
			&branches_bk_pin_key(source_branch_id),
			&pin_versionstamp,
			MutationType::ByteMin,
		);
		Ok(())
	})
	.await?;

	let err = db
		.run(move |tx| async move {
			branch::derive_branch_at(
				&tx,
				source_branch_id,
				at_versionstamp,
				new_branch_id,
				namespace_branch_id,
				None,
			)
			.await
		})
		.await
		.expect_err("fork should be outside retention");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::ForkOutOfRetention
			))
	);
	assert!(
		read_value(&db, branches_list_key(new_branch_id))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn derive_branch_at_enforces_max_fork_depth() -> Result<()> {
	let db = test_db().await?;
	let source_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x4444));
	let new_branch_id = ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x5555));
	let namespace_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0x6666));

	seed_branch_record(
		&db,
		source_branch_id,
		namespace_branch_id,
		sqlite_storage::constants::MAX_FORK_DEPTH,
	)
	.await?;

	let err = db
		.run(move |tx| async move {
			branch::derive_branch_at(
				&tx,
				source_branch_id,
				[1; 16],
				new_branch_id,
				namespace_branch_id,
				None,
			)
			.await
		})
		.await
		.expect_err("fork depth should be capped");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::ForkChainTooDeep
			))
	);
	assert!(
		read_value(&db, branches_list_key(new_branch_id))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn derive_namespace_branch_at_writes_branch_metadata_without_tier_state() -> Result<()> {
	let db = test_db().await?;
	let source_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0x7777));
	let new_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0x8888));
	let at_versionstamp = [3; 16];

	seed_namespace_branch_record(&db, source_branch_id, 0).await?;

	db.run(move |tx| async move {
		branch::derive_namespace_branch_at(
			&tx,
			source_branch_id,
			at_versionstamp,
			new_branch_id,
			None,
		)
		.await
	})
	.await?;

	let record_bytes = read_value(&db, namespace_branches_list_key(new_branch_id))
		.await?
		.expect("derived namespace branch record should exist");
	let record = decode_namespace_branch_record(&record_bytes)?;
	assert_eq!(record.branch_id, new_branch_id);
	assert_eq!(record.parent, Some(source_branch_id));
	assert_eq!(record.parent_versionstamp, Some(at_versionstamp));
	assert_eq!(record.root_versionstamp, at_versionstamp);
	assert_eq!(record.fork_depth, 1);
	assert_eq!(record.created_from_bookmark, None);
	assert_eq!(record.state, BranchState::Live);
	assert_eq!(read_namespace_refcount(&db, source_branch_id).await?, 2);
	assert_eq!(read_namespace_refcount(&db, new_branch_id).await?, 1);
	assert_eq!(
		read_value(&db, namespace_branches_desc_pin_key(source_branch_id)).await?,
		Some(at_versionstamp.to_vec())
	);
	assert!(
		read_value(&db, namespace_branches_tier_state_key(new_branch_id))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn derive_namespace_branch_at_rejects_expired_pin_before_writing_record() -> Result<()> {
	let db = test_db().await?;
	let source_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0x9999));
	let new_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0xaaaa));
	let at_versionstamp = [1; 16];
	let pin_versionstamp = [2; 16];

	seed_namespace_branch_record(&db, source_branch_id, 0).await?;
	db.run(move |tx| async move {
		tx.informal().atomic_op(
			&namespace_branches_bk_pin_key(source_branch_id),
			&pin_versionstamp,
			MutationType::ByteMin,
		);
		Ok(())
	})
	.await?;

	let err = db
		.run(move |tx| async move {
			branch::derive_namespace_branch_at(
				&tx,
				source_branch_id,
				at_versionstamp,
				new_branch_id,
				None,
			)
			.await
		})
		.await
		.expect_err("namespace fork should be outside retention");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::ForkOutOfRetention
			))
	);
	assert!(
		read_value(&db, namespace_branches_list_key(new_branch_id))
			.await?
			.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn derive_namespace_branch_at_enforces_max_namespace_depth() -> Result<()> {
	let db = test_db().await?;
	let source_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0xbbbb));
	let new_branch_id = NamespaceBranchId::from_uuid(uuid::Uuid::from_u128(0xcccc));

	seed_namespace_branch_record(
		&db,
		source_branch_id,
		sqlite_storage::constants::MAX_NAMESPACE_DEPTH,
	)
	.await?;

	let err = db
		.run(move |tx| async move {
			branch::derive_namespace_branch_at(&tx, source_branch_id, [1; 16], new_branch_id, None)
				.await
		})
		.await
		.expect_err("namespace fork depth should be capped");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::NamespaceForkChainTooDeep
			))
	);
	assert!(
		read_value(&db, namespace_branches_list_key(new_branch_id))
			.await?
			.is_none()
	);

	Ok(())
}

async fn seed_branch_record(
	db: &universaldb::Database,
	branch_id: ActorBranchId,
	namespace_branch: NamespaceBranchId,
	fork_depth: u8,
) -> Result<()> {
	let record = ActorBranchRecord {
		branch_id,
		namespace_branch,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [0; 16],
		fork_depth,
		created_at_ms: 1_000,
		created_from_bookmark: None,
		state: BranchState::Live,
	};
	let encoded_record = encode_actor_branch_record(record)?;
	db.run(move |tx| {
		let encoded_record = encoded_record.clone();
		async move {
			tx.informal()
				.set(&branches_list_key(branch_id), &encoded_record);
			Ok(())
		}
	})
	.await
}

async fn seed_namespace_branch_record(
	db: &universaldb::Database,
	branch_id: NamespaceBranchId,
	fork_depth: u8,
) -> Result<()> {
	let record = NamespaceBranchRecord {
		branch_id,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [0; 16],
		fork_depth,
		created_at_ms: 1_000,
		created_from_bookmark: None,
		state: BranchState::Live,
	};
	let encoded_record = encode_namespace_branch_record(record)?;
	db.run(move |tx| {
		let encoded_record = encoded_record.clone();
		async move {
			tx.informal()
				.set(&namespace_branches_list_key(branch_id), &encoded_record);
			tx.informal().atomic_op(
				&namespace_branches_refcount_key(branch_id),
				&1_i64.to_le_bytes(),
				MutationType::Add,
			);
			Ok(())
		}
	})
	.await
}
