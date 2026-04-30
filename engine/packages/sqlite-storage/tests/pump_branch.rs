use std::sync::Arc;

use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	keys::{
		actor_pointer_cur_key, actor_pointer_history_prefix, branch_commit_key,
		branch_meta_head_at_fork_key, branch_meta_head_key, branches_bk_pin_key,
		branches_desc_pin_key, branches_list_key, branches_refcount_key,
		namespace_branches_actor_tombstone_key, namespace_branches_bk_pin_key,
		namespace_branches_desc_pin_key, namespace_branches_list_key,
		namespace_branches_refcount_key, namespace_pointer_cur_key,
		namespace_pointer_history_prefix,
	},
	pump::{ActorDb, branch},
	types::{
		ActorBranchId, ActorBranchRecord, BranchState, DBHead, DirtyPage, NamespaceBranchId,
		NamespaceBranchRecord, NamespaceId, ResolvedVersionstamp,
		decode_actor_branch_record, decode_actor_pointer, decode_commit_row, decode_db_head,
		decode_namespace_branch_record, decode_namespace_pointer, encode_actor_branch_record,
		encode_namespace_branch_record,
	},
};
use tempfile::Builder;
use universaldb::{
	RangeOption,
	options::{MutationType, StreamingMode},
	utils::IsolationLevel::Snapshot,
};
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

const TEST_ACTOR: &str = "test-actor";

fn test_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn target_namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
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

fn page_bytes(fill: u8) -> Vec<u8> {
	vec![fill; sqlite_storage::keys::PAGE_SIZE as usize]
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

async fn clear_value(db: &universaldb::Database, key: Vec<u8>) -> Result<()> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			tx.informal().clear(&key);
			Ok(())
		}
	})
	.await
}

async fn read_prefix_values(
	db: &universaldb::Database,
	prefix: Vec<u8>,
) -> Result<Vec<Vec<u8>>> {
	db.run(move |tx| {
		let prefix = prefix.clone();
		async move {
			let prefix_subspace = universaldb::Subspace::from(
				universaldb::tuple::Subspace::from_bytes(prefix),
			);
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..RangeOption::from(&prefix_subspace)
				},
				Snapshot,
			);
			let mut values = Vec::new();

			while let Some(entry) = stream.try_next().await? {
				values.push(entry.value().to_vec());
			}

			Ok(values)
		}
	})
	.await
}

async fn read_namespace_branch_id(db: &universaldb::Database) -> Result<NamespaceBranchId> {
	read_namespace_branch_id_for(db, test_namespace()).await
}

async fn read_namespace_branch_id_for(
	db: &universaldb::Database,
	namespace_id: Id,
) -> Result<NamespaceBranchId> {
	let namespace_id = NamespaceId::from_gas_id(namespace_id);
	let namespace_pointer_bytes = read_value(db, namespace_pointer_cur_key(namespace_id))
		.await?
		.expect("namespace pointer should exist");

	Ok(decode_namespace_pointer(&namespace_pointer_bytes)?.current_branch)
}

async fn read_actor_branch_id(
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

async fn read_branch_id(db: &universaldb::Database) -> Result<ActorBranchId> {
	read_actor_branch_id(db, test_namespace(), TEST_ACTOR).await
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

async fn read_namespace_pin(
	db: &universaldb::Database,
	branch_id: NamespaceBranchId,
) -> Result<[u8; 16]> {
	let bytes = read_value(db, namespace_branches_desc_pin_key(branch_id))
		.await?
		.expect("namespace branch desc pin should exist");

	Ok(bytes
		.as_slice()
		.try_into()
		.expect("namespace branch desc pin should be 16 bytes"))
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
async fn derive_namespace_branch_at_writes_branch_metadata() -> Result<()> {
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

#[tokio::test]
async fn fork_namespace_writes_pointer_and_metadata_without_eager_aptr() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_namespace_id = NamespaceId::from_gas_id(test_namespace());
	let source_namespace_branch = read_namespace_branch_id(&db).await?;
	let source_actor_branch = read_branch_id(&db).await?;
	let source_commit_bytes = read_value(&db, branch_commit_key(source_actor_branch, 1))
		.await?
		.expect("source commit row should exist");
	let source_commit = decode_commit_row(&source_commit_bytes)?;

	let forked_namespace_id = branch::fork_namespace(
		&db,
		source_namespace_id,
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	assert_ne!(forked_namespace_id, source_namespace_id);
	let pointer_bytes = read_value(&db, namespace_pointer_cur_key(forked_namespace_id))
		.await?
		.expect("forked namespace pointer should exist");
	let forked_pointer = decode_namespace_pointer(&pointer_bytes)?;
	let forked_branch_record_bytes =
		read_value(&db, namespace_branches_list_key(forked_pointer.current_branch))
			.await?
			.expect("forked namespace branch record should exist");
	let forked_record = decode_namespace_branch_record(&forked_branch_record_bytes)?;
	assert_eq!(forked_record.parent, Some(source_namespace_branch));
	assert_eq!(
		forked_record.parent_versionstamp,
		Some(source_commit.versionstamp)
	);
	assert_eq!(forked_record.fork_depth, 1);
	assert_eq!(forked_record.state, BranchState::Live);
	assert_eq!(read_namespace_refcount(&db, source_namespace_branch).await?, 2);
	assert_eq!(
		read_namespace_refcount(&db, forked_pointer.current_branch).await?,
		1
	);
	assert_eq!(
		read_namespace_pin(&db, source_namespace_branch).await?,
		source_commit.versionstamp
	);
	assert!(
		read_value(
			&db,
			actor_pointer_cur_key(forked_pointer.current_branch, TEST_ACTOR)
		)
		.await?
		.is_none()
	);

	Ok(())
}

#[tokio::test]
async fn fork_namespace_enforces_max_namespace_depth() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_actor_branch = read_branch_id(&db).await?;
	let source_commit_bytes = read_value(&db, branch_commit_key(source_actor_branch, 1))
		.await?
		.expect("source commit row should exist");
	let source_commit = decode_commit_row(&source_commit_bytes)?;
	let at = ResolvedVersionstamp {
		versionstamp: source_commit.versionstamp,
		bookmark: None,
	};
	let mut source_namespace_id = NamespaceId::from_gas_id(test_namespace());

	for _ in 0..sqlite_storage::constants::MAX_NAMESPACE_DEPTH {
		source_namespace_id = branch::fork_namespace(&db, source_namespace_id, at.clone()).await?;
	}

	let err = branch::fork_namespace(&db, source_namespace_id, at)
		.await
		.expect_err("namespace fork depth should be capped");

	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::NamespaceForkChainTooDeep
			))
	);

	Ok(())
}

#[tokio::test]
async fn resolve_actor_pointer_walks_namespace_parent_chain_after_fork() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_actor_branch = read_branch_id(&db).await?;
	let source_commit_bytes = read_value(&db, branch_commit_key(source_actor_branch, 1))
		.await?
		.expect("source commit row should exist");
	let source_commit = decode_commit_row(&source_commit_bytes)?;

	let forked_namespace_id = branch::fork_namespace(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;
	let pointer_bytes = read_value(&db, namespace_pointer_cur_key(forked_namespace_id))
		.await?
		.expect("forked namespace pointer should exist");
	let forked_pointer = decode_namespace_pointer(&pointer_bytes)?;

	let resolved_pointer = db
		.run(move |tx| async move {
			branch::resolve_actor_pointer(&tx, forked_pointer.current_branch, TEST_ACTOR, Snapshot)
				.await
		})
		.await?
		.expect("actor pointer should be inherited from parent namespace branch");
	assert_eq!(resolved_pointer.current_branch, source_actor_branch);

	let forked_actor_db = ActorDb::new(
		db,
		test_ups(),
		Id::v1(forked_namespace_id.as_uuid(), 1),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	let pages = forked_actor_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

	Ok(())
}

#[tokio::test]
async fn resolve_actor_pointer_honors_namespace_branch_tombstones() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_actor_branch = read_branch_id(&db).await?;
	let source_commit_bytes = read_value(&db, branch_commit_key(source_actor_branch, 1))
		.await?
		.expect("source commit row should exist");
	let source_commit = decode_commit_row(&source_commit_bytes)?;

	let forked_namespace_id = branch::fork_namespace(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;
	let pointer_bytes = read_value(&db, namespace_pointer_cur_key(forked_namespace_id))
		.await?
		.expect("forked namespace pointer should exist");
	let forked_pointer = decode_namespace_pointer(&pointer_bytes)?;
	let forked_namespace_branch = forked_pointer.current_branch;

	db.run(move |tx| async move {
		tx.informal().set(
			&namespace_branches_actor_tombstone_key(forked_namespace_branch, TEST_ACTOR),
			&[],
		);
		Ok(())
	})
	.await?;

	let err = db
		.run(move |tx| async move {
			branch::resolve_actor_pointer(&tx, forked_namespace_branch, TEST_ACTOR, Snapshot).await
		})
		.await
		.expect_err("tombstone should block inherited actor pointer");
	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::ActorNotFound
			))
	);

	let forked_actor_db = ActorDb::new(
		db,
		test_ups(),
		Id::v1(forked_namespace_id.as_uuid(), 1),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	let err = forked_actor_db
		.get_pages(vec![1])
		.await
		.expect_err("tombstoned actor should not fall back to legacy storage");
	assert!(
		err.downcast_ref::<sqlite_storage::error::SqliteStorageError>()
			.is_some_and(|err| matches!(
				err,
				sqlite_storage::error::SqliteStorageError::ActorNotFound
			))
	);

	Ok(())
}

#[tokio::test]
async fn fork_actor_writes_target_pointer_and_reads_source_data() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let source_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	source_actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_branch_id = read_branch_id(&db).await?;
	let source_commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
		.await?
		.expect("source commit row should exist");
	let source_commit = decode_commit_row(&source_commit_bytes)?;

	let target_seed = ActorDb::new(
		db.clone(),
		test_ups(),
		target_namespace(),
		"target-seed".to_string(),
		NodeId::new(),
	);
	target_seed.commit(vec![page(1, 0xaa)], 1, 1_100).await?;
	let target_namespace_branch = read_namespace_branch_id_for(&db, target_namespace()).await?;

	let forked_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(target_namespace()),
	)
	.await?;

	assert_ne!(forked_actor_id, TEST_ACTOR);
	let forked_branch_id = read_actor_branch_id(&db, target_namespace(), &forked_actor_id).await?;
	let forked_record_bytes = read_value(&db, branches_list_key(forked_branch_id))
		.await?
		.expect("forked actor branch record should exist");
	let forked_record = decode_actor_branch_record(&forked_record_bytes)?;
	assert_eq!(forked_record.namespace_branch, target_namespace_branch);
	assert_eq!(forked_record.parent, Some(source_branch_id));
	assert_eq!(
		forked_record.parent_versionstamp,
		Some(source_commit.versionstamp)
	);
	assert_eq!(read_refcount(&db, source_branch_id).await?, 2);
	assert_eq!(read_refcount(&db, forked_branch_id).await?, 1);

	let forked_actor_db = ActorDb::new(
		db,
		test_ups(),
		target_namespace(),
		forked_actor_id,
		NodeId::new(),
	);
	let pages = forked_actor_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

	Ok(())
}

#[tokio::test]
async fn fork_actor_can_use_depth_one_source_branch() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let root_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	root_actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_branch_id = read_branch_id(&db).await?;
	let root_commit_bytes = read_value(&db, branch_commit_key(root_branch_id, 1))
		.await?
		.expect("root commit row should exist");
	let root_commit = decode_commit_row(&root_commit_bytes)?;

	let depth_one_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: root_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await?;
	let depth_one_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		depth_one_actor_id.clone(),
		NodeId::new(),
	);
	depth_one_actor_db
		.commit(vec![page(2, 0x22)], 3, 2_000)
		.await?;
	let depth_one_branch_id =
		read_actor_branch_id(&db, test_namespace(), &depth_one_actor_id).await?;
	let depth_one_commit_bytes = read_value(&db, branch_commit_key(depth_one_branch_id, 1))
		.await?
		.expect("depth-one commit row should exist");
	let depth_one_commit = decode_commit_row(&depth_one_commit_bytes)?;

	let depth_two_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		depth_one_actor_id,
		ResolvedVersionstamp {
			versionstamp: depth_one_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await?;
	let depth_two_branch_id =
		read_actor_branch_id(&db, test_namespace(), &depth_two_actor_id).await?;
	let depth_two_record_bytes = read_value(&db, branches_list_key(depth_two_branch_id))
		.await?
		.expect("depth-two branch record should exist");
	let depth_two_record = decode_actor_branch_record(&depth_two_record_bytes)?;
	assert_eq!(depth_two_record.parent, Some(depth_one_branch_id));
	assert_eq!(depth_two_record.fork_depth, 2);

	let depth_two_actor_db = ActorDb::new(
		db,
		test_ups(),
		test_namespace(),
		depth_two_actor_id,
		NodeId::new(),
	);
	let pages = depth_two_actor_db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
	assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

	Ok(())
}

#[tokio::test]
async fn actor_db_reuses_cached_branch_ancestry_for_reads() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let root_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	root_actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_branch_id = read_branch_id(&db).await?;
	let root_commit = decode_commit_row(
		&read_value(&db, branch_commit_key(root_branch_id, 1))
			.await?
			.expect("root commit row should exist"),
	)?;

	let child_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: root_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await?;
	let child_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		child_actor_id.clone(),
		NodeId::new(),
	);
	child_actor_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;
	let child_branch_id = read_actor_branch_id(&db, test_namespace(), &child_actor_id).await?;
	let child_commit = decode_commit_row(
		&read_value(&db, branch_commit_key(child_branch_id, 1))
			.await?
			.expect("child commit row should exist"),
	)?;

	let grandchild_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		child_actor_id,
		ResolvedVersionstamp {
			versionstamp: child_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await?;
	let grandchild_branch_id =
		read_actor_branch_id(&db, test_namespace(), &grandchild_actor_id).await?;
	let grandchild_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		grandchild_actor_id,
		NodeId::new(),
	);

	let pages = grandchild_actor_db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
	assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

	clear_value(&db, branches_list_key(grandchild_branch_id)).await?;
	clear_value(&db, branches_list_key(child_branch_id)).await?;
	clear_value(&db, branches_list_key(root_branch_id)).await?;

	let pages = grandchild_actor_db.get_pages(vec![1, 2]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
	assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

	Ok(())
}

#[tokio::test]
async fn fork_actor_can_use_deep_source_branch() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let root_actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	root_actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let mut source_actor_id = TEST_ACTOR.to_string();
	let mut source_branch_id = read_branch_id(&db).await?;
	let mut source_commit = decode_commit_row(
		&read_value(&db, branch_commit_key(source_branch_id, 1))
			.await?
			.expect("root commit row should exist"),
	)?;

	for depth in 1..=3 {
		let forked_actor_id = branch::fork_actor(
			&db,
			NamespaceId::from_gas_id(test_namespace()),
			source_actor_id,
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				bookmark: None,
			},
			NamespaceId::from_gas_id(test_namespace()),
		)
		.await?;
		let forked_actor_db = ActorDb::new(
			db.clone(),
			test_ups(),
			test_namespace(),
			forked_actor_id.clone(),
			NodeId::new(),
		);
		forked_actor_db
			.commit(vec![page(depth + 1, 0x20 + depth as u8)], depth + 2, 2_000 + depth as i64)
			.await?;
		source_actor_id = forked_actor_id;
		source_branch_id = read_actor_branch_id(&db, test_namespace(), &source_actor_id).await?;
		let commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
			.await?
			.expect("fork commit row should exist");
		source_commit = decode_commit_row(&commit_bytes)?;
	}

	let final_actor_id = branch::fork_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		source_actor_id,
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await?;
	let final_branch_id = read_actor_branch_id(&db, test_namespace(), &final_actor_id).await?;
	let final_record_bytes = read_value(&db, branches_list_key(final_branch_id))
		.await?
		.expect("final branch record should exist");
	let final_record = decode_actor_branch_record(&final_record_bytes)?;
	assert_eq!(final_record.fork_depth, 4);

	let final_actor_db = ActorDb::new(
		db,
		test_ups(),
		test_namespace(),
		final_actor_id,
		NodeId::new(),
	);
	let pages = final_actor_db.get_pages(vec![1, 2, 3, 4]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
	assert_eq!(pages[1].bytes, Some(page_bytes(0x21)));
	assert_eq!(pages[2].bytes, Some(page_bytes(0x22)));
	assert_eq!(pages[3].bytes, Some(page_bytes(0x23)));

	Ok(())
}

#[tokio::test]
async fn rollback_actor_freezes_old_branch_and_swaps_pointer() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	actor_db.commit(vec![page(1, 0x22)], 2, 2_000).await?;
	let old_branch_id = read_branch_id(&db).await?;
	let namespace_branch_id = read_namespace_branch_id(&db).await?;
	let first_commit = decode_commit_row(
		&read_value(&db, branch_commit_key(old_branch_id, 1))
			.await?
			.expect("first commit row should exist"),
	)?;
	let pages = actor_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x22)));

	let rolled_branch_id = branch::rollback_actor(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: first_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	assert_ne!(rolled_branch_id, old_branch_id);
	let current_branch_id = read_branch_id(&db).await?;
	assert_eq!(current_branch_id, rolled_branch_id);

	let old_record_bytes = read_value(&db, branches_list_key(old_branch_id))
		.await?
		.expect("old branch record should exist");
	let old_record = decode_actor_branch_record(&old_record_bytes)?;
	assert_eq!(old_record.state, BranchState::Frozen);
	assert_eq!(read_refcount(&db, old_branch_id).await?, 1);
	assert_eq!(read_refcount(&db, rolled_branch_id).await?, 1);

	let rolled_record_bytes = read_value(&db, branches_list_key(rolled_branch_id))
		.await?
		.expect("rolled branch record should exist");
	let rolled_record = decode_actor_branch_record(&rolled_record_bytes)?;
	assert_eq!(rolled_record.parent, Some(old_branch_id));
	assert_eq!(
		rolled_record.parent_versionstamp,
		Some(first_commit.versionstamp)
	);
	assert_eq!(rolled_record.state, BranchState::Live);

	let history_values = read_prefix_values(
		&db,
		actor_pointer_history_prefix(namespace_branch_id, TEST_ACTOR),
	)
	.await?;
	assert_eq!(history_values.len(), 1);
	let history_pointer = decode_actor_pointer(&history_values[0])?;
	assert_eq!(history_pointer.current_branch, old_branch_id);

	actor_db.commit(vec![page(2, 0x33)], 3, 3_000).await?;

	let new_head_bytes = read_value(&db, branch_meta_head_key(rolled_branch_id))
		.await?
		.expect("rolled branch head should exist after commit");
	let new_head = decode_db_head(&new_head_bytes)?;
	assert_eq!(new_head.branch_id, rolled_branch_id);
	assert_eq!(new_head.db_size_pages, 3);
	let old_head_bytes = read_value(&db, branch_meta_head_key(old_branch_id))
		.await?
		.expect("old branch head should still exist");
	let old_head = decode_db_head(&old_head_bytes)?;
	assert_eq!(old_head.head_txid, 2);

	Ok(())
}

#[tokio::test]
async fn rollback_namespace_freezes_old_branch_and_swaps_pointer() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = ActorDb::new(
		db.clone(),
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	actor_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let old_namespace_branch_id = read_namespace_branch_id(&db).await?;
	let actor_branch_id = read_branch_id(&db).await?;
	let first_commit = decode_commit_row(
		&read_value(&db, branch_commit_key(actor_branch_id, 1))
			.await?
			.expect("first commit row should exist"),
	)?;

	let rolled_namespace_branch_id = branch::rollback_namespace(
		&db,
		NamespaceId::from_gas_id(test_namespace()),
		ResolvedVersionstamp {
			versionstamp: first_commit.versionstamp,
			bookmark: None,
		},
	)
	.await?;

	assert_ne!(rolled_namespace_branch_id, old_namespace_branch_id);
	let current_namespace_branch_id = read_namespace_branch_id(&db).await?;
	assert_eq!(current_namespace_branch_id, rolled_namespace_branch_id);

	let old_record_bytes = read_value(&db, namespace_branches_list_key(old_namespace_branch_id))
		.await?
		.expect("old namespace branch record should exist");
	let old_record = decode_namespace_branch_record(&old_record_bytes)?;
	assert_eq!(old_record.state, BranchState::Frozen);
	assert_eq!(read_namespace_refcount(&db, old_namespace_branch_id).await?, 1);
	assert_eq!(
		read_namespace_refcount(&db, rolled_namespace_branch_id).await?,
		1
	);

	let rolled_record_bytes = read_value(&db, namespace_branches_list_key(rolled_namespace_branch_id))
		.await?
		.expect("rolled namespace branch record should exist");
	let rolled_record = decode_namespace_branch_record(&rolled_record_bytes)?;
	assert_eq!(rolled_record.parent, Some(old_namespace_branch_id));
	assert_eq!(
		rolled_record.parent_versionstamp,
		Some(first_commit.versionstamp)
	);
	assert_eq!(rolled_record.state, BranchState::Live);

	let history_values = read_prefix_values(
		&db,
		namespace_pointer_history_prefix(NamespaceId::from_gas_id(test_namespace())),
	)
	.await?;
	assert_eq!(history_values.len(), 1);
	let history_pointer = decode_namespace_pointer(&history_values[0])?;
	assert_eq!(history_pointer.current_branch, old_namespace_branch_id);

	let rolled_actor_db = ActorDb::new(
		db,
		test_ups(),
		test_namespace(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	);
	let pages = rolled_actor_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

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
