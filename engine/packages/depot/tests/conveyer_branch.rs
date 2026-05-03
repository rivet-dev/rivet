mod common;

use anyhow::Result;
use depot::{
	conveyer::{branch, history_pin},
	keys::{
		branch_commit_key, branch_meta_head_at_fork_key, branch_meta_head_key, branch_shard_key,
		branch_vtx_key, branches_desc_pin_key, branches_list_key, branches_refcount_key,
		branches_restore_point_pin_key, bucket_branches_database_name_tombstone_key,
		bucket_branches_desc_pin_key, bucket_branches_list_key, bucket_branches_refcount_key,
		bucket_branches_restore_point_pin_key, bucket_pointer_cur_key,
		bucket_pointer_history_prefix, database_pointer_cur_key, database_pointer_history_prefix,
		db_pin_key,
	},
	ltx::{LtxHeader, encode_ltx_v3},
	types::{
		BranchState, BucketBranchId, BucketBranchRecord, BucketId, CommitRow, DBHead,
		DatabaseBranchId, DatabaseBranchRecord, DbHistoryPinKind, DirtyPage, ResolvedVersionstamp,
		decode_bucket_branch_record, decode_bucket_pointer, decode_commit_row,
		decode_database_branch_record, decode_database_pointer, decode_db_head,
		decode_db_history_pin, encode_bucket_branch_record, encode_commit_row,
		encode_database_branch_record,
	},
};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use universaldb::{
	RangeOption,
	options::{MutationType, StreamingMode},
	utils::IsolationLevel::Snapshot,
};

const TEST_DATABASE: &str = "test-database";

fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

fn target_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
}

fn page_bytes(fill: u8) -> Vec<u8> {
	vec![fill; depot::keys::PAGE_SIZE as usize]
}

fn encoded_blob(txid: u64, pages: Vec<DirtyPage>) -> Result<Vec<u8>> {
	encode_ltx_v3(LtxHeader::delta(txid, 1, 999), &pages)
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

async fn read_prefix_values(db: &universaldb::Database, prefix: Vec<u8>) -> Result<Vec<Vec<u8>>> {
	db.run(move |tx| {
		let prefix = prefix.clone();
		async move {
			let prefix_subspace =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
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

async fn read_bucket_branch_id(db: &universaldb::Database) -> Result<BucketBranchId> {
	read_bucket_branch_id_for(db, test_bucket()).await
}

async fn read_bucket_branch_id_for(
	db: &universaldb::Database,
	bucket_id: Id,
) -> Result<BucketBranchId> {
	let bucket_id = BucketId::from_gas_id(bucket_id);
	let bucket_pointer_bytes = read_value(db, bucket_pointer_cur_key(bucket_id))
		.await?
		.expect("bucket pointer should exist");

	Ok(decode_bucket_pointer(&bucket_pointer_bytes)?.current_branch)
}

async fn read_database_branch_id(
	db: &universaldb::Database,
	bucket_id: Id,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	let bucket_branch = read_bucket_branch_id_for(db, bucket_id).await?;
	let bytes = read_value(db, database_pointer_cur_key(bucket_branch, database_id))
		.await?
		.expect("database pointer should exist");

	Ok(decode_database_pointer(&bytes)?.current_branch)
}

async fn read_branch_id(db: &universaldb::Database) -> Result<DatabaseBranchId> {
	read_database_branch_id(db, test_bucket(), TEST_DATABASE).await
}

async fn read_branch_head(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<DBHead> {
	let bytes = read_value(db, branch_meta_head_key(branch_id))
		.await?
		.expect("branch head should exist");

	decode_db_head(&bytes)
}

async fn read_head_commit(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<CommitRow> {
	let head = read_branch_head(db, branch_id).await?;
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

async fn read_bucket_refcount(
	db: &universaldb::Database,
	branch_id: BucketBranchId,
) -> Result<i64> {
	let bytes = read_value(db, bucket_branches_refcount_key(branch_id))
		.await?
		.expect("bucket branch refcount should exist");
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.expect("bucket branch refcount should be i64 LE");

	Ok(i64::from_le_bytes(bytes))
}

async fn read_bucket_pin(
	db: &universaldb::Database,
	branch_id: BucketBranchId,
) -> Result<[u8; 16]> {
	let bytes = read_value(db, bucket_branches_desc_pin_key(branch_id))
		.await?
		.expect("bucket branch desc pin should exist");

	Ok(bytes
		.as_slice()
		.try_into()
		.expect("bucket branch desc pin should be 16 bytes"))
}

macro_rules! branch_matrix {
	($prefix:expr, |$ctx:ident, $db:ident, $database_db:ident| $body:block) => {
		common::test_matrix($prefix, |_tier, $ctx| {
			Box::pin(async move {
				let $db = $ctx.udb.clone();
				let $database_db = $ctx.make_db(test_bucket(), TEST_DATABASE);
				$body
			})
		})
		.await
	};
}

macro_rules! udb_matrix {
	($prefix:expr, |$ctx:ident, $db:ident| $body:block) => {
		common::test_matrix($prefix, |_tier, $ctx| {
			Box::pin(async move {
				let $db = $ctx.udb.clone();
				$body
			})
		})
		.await
	};
}

#[tokio::test]
async fn derive_branch_at_snapshots_head_and_writes_branch_metadata() -> Result<()> {
	branch_matrix!("depot-branch-derive-snapshot", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		database_db.commit(vec![page(2, 0x22)], 3, 2_000).await?;
		let source_branch_id = read_branch_id(&db).await?;
		let bucket_branch_id = read_bucket_branch_id(&db).await?;
		let first_commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
			.await?
			.expect("first commit row should exist");
		let first_commit = decode_commit_row(&first_commit_bytes)?;
		let new_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x9999));

		db.run(move |tx| async move {
			branch::derive_branch_at(
				&tx,
				source_branch_id,
				first_commit.versionstamp,
				new_branch_id,
				bucket_branch_id,
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
		let record = decode_database_branch_record(&record_bytes)?;
		assert_eq!(record.branch_id, new_branch_id);
		assert_eq!(record.bucket_branch, bucket_branch_id);
		assert_eq!(record.parent, Some(source_branch_id));
		assert_eq!(record.parent_versionstamp, Some(first_commit.versionstamp));
		assert_eq!(record.root_versionstamp, first_commit.versionstamp);
		assert_eq!(record.fork_depth, 1);
		assert_eq!(record.created_from_restore_point, None);
		assert_eq!(record.state, BranchState::Live);
		assert_eq!(read_refcount(&db, source_branch_id).await?, 2);
		assert_eq!(read_refcount(&db, new_branch_id).await?, 1);
		assert_eq!(
			read_value(&db, branches_desc_pin_key(source_branch_id)).await?,
			Some(first_commit.versionstamp.to_vec())
		);
		let pin_bytes = read_value(
			&db,
			db_pin_key(
				source_branch_id,
				&history_pin::database_fork_pin_id(new_branch_id),
			),
		)
		.await?
		.expect("database fork DB_PIN should exist");
		let pin = decode_db_history_pin(&pin_bytes)?;
		assert_eq!(pin.kind, DbHistoryPinKind::DatabaseFork);
		assert_eq!(pin.at_txid, 1);
		assert_eq!(pin.at_versionstamp, first_commit.versionstamp);
		assert_eq!(pin.owner_database_branch_id, Some(new_branch_id));

		Ok(())
	})
}

#[tokio::test]
async fn derive_branch_at_rejects_expired_pin_before_copying_head() -> Result<()> {
	udb_matrix!("depot-branch-expired-pin", |ctx, db| {
		let source_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x1111));
		let new_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x2222));
		let bucket_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x3333));
		let at_versionstamp = [1; 16];
		let pin_versionstamp = [2; 16];

		seed_branch_record(&db, source_branch_id, bucket_branch_id, 0).await?;
		db.run(move |tx| async move {
			tx.informal().atomic_op(
				&branches_restore_point_pin_key(source_branch_id),
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
					bucket_branch_id,
					None,
				)
				.await
			})
			.await
			.expect_err("fork should be outside retention");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::ForkOutOfRetention
				))
		);
		assert!(
			read_value(&db, branches_list_key(new_branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn derive_branch_at_rejects_versionstamp_below_parent_gc_floor() -> Result<()> {
	udb_matrix!("depot-branch-parent-gc-floor", |ctx, db| {
		let source_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x1111));
		let new_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x2222));
		let bucket_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x3333));
		let old_versionstamp = [1; 16];
		let gc_floor_versionstamp = [2; 16];

		db.run(move |tx| async move {
			tx.informal().set(
				&branches_list_key(source_branch_id),
				&encode_database_branch_record(DatabaseBranchRecord {
					branch_id: source_branch_id,
					bucket_branch: bucket_branch_id,
					parent: None,
					parent_versionstamp: None,
					root_versionstamp: gc_floor_versionstamp,
					fork_depth: 0,
					created_at_ms: 1_000,
					created_from_restore_point: None,
					state: BranchState::Live,
					lifecycle_generation: 0,
				})?,
			);
			tx.informal().set(
				&branches_refcount_key(source_branch_id),
				&1_i64.to_le_bytes(),
			);
			tx.informal().set(
				&branch_vtx_key(source_branch_id, old_versionstamp),
				&1_u64.to_be_bytes(),
			);
			tx.informal().set(
				&branch_commit_key(source_branch_id, 1),
				&encode_commit_row(CommitRow {
					wall_clock_ms: 1_000,
					versionstamp: old_versionstamp,
					db_size_pages: 1,
					post_apply_checksum: 1,
				})?,
			);
			Ok(())
		})
		.await?;

		let err = db
			.run(move |tx| async move {
				branch::derive_branch_at(
					&tx,
					source_branch_id,
					old_versionstamp,
					new_branch_id,
					bucket_branch_id,
					None,
				)
				.await
			})
			.await
			.expect_err("fork should be rejected below the parent GC floor");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::ForkOutOfRetention
				)),
			"unexpected error: {err:?}"
		);
		assert!(
			read_value(&db, branches_list_key(new_branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn derive_branch_at_enforces_max_fork_depth() -> Result<()> {
	udb_matrix!("depot-branch-max-fork-depth", |ctx, db| {
		let source_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x4444));
		let new_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x5555));
		let bucket_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x6666));

		seed_branch_record(
			&db,
			source_branch_id,
			bucket_branch_id,
			depot::constants::MAX_FORK_DEPTH,
		)
		.await?;

		let err = db
			.run(move |tx| async move {
				branch::derive_branch_at(
					&tx,
					source_branch_id,
					[1; 16],
					new_branch_id,
					bucket_branch_id,
					None,
				)
				.await
			})
			.await
			.expect_err("fork depth should be capped");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::ForkChainTooDeep
				))
		);
		assert!(
			read_value(&db, branches_list_key(new_branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn derive_bucket_branch_at_writes_branch_metadata() -> Result<()> {
	udb_matrix!("depot-branch-bucket-derive", |ctx, db| {
		let source_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x7777));
		let new_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x8888));
		let at_versionstamp = [3; 16];

		seed_bucket_branch_record(&db, source_branch_id, 0).await?;

		db.run(move |tx| async move {
			branch::derive_bucket_branch_at(
				&tx,
				source_branch_id,
				at_versionstamp,
				new_branch_id,
				None,
			)
			.await
		})
		.await?;

		let record_bytes = read_value(&db, bucket_branches_list_key(new_branch_id))
			.await?
			.expect("derived bucket branch record should exist");
		let record = decode_bucket_branch_record(&record_bytes)?;
		assert_eq!(record.branch_id, new_branch_id);
		assert_eq!(record.parent, Some(source_branch_id));
		assert_eq!(record.parent_versionstamp, Some(at_versionstamp));
		assert_eq!(record.root_versionstamp, at_versionstamp);
		assert_eq!(record.fork_depth, 1);
		assert_eq!(record.created_from_restore_point, None);
		assert_eq!(record.state, BranchState::Live);
		assert_eq!(read_bucket_refcount(&db, source_branch_id).await?, 2);
		assert_eq!(read_bucket_refcount(&db, new_branch_id).await?, 1);
		assert_eq!(
			read_value(&db, bucket_branches_desc_pin_key(source_branch_id)).await?,
			Some(at_versionstamp.to_vec())
		);

		Ok(())
	})
}

#[tokio::test]
async fn derive_bucket_branch_at_rejects_expired_pin_before_writing_record() -> Result<()> {
	udb_matrix!("depot-branch-bucket-expired-pin", |ctx, db| {
		let source_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0x9999));
		let new_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0xaaaa));
		let at_versionstamp = [1; 16];
		let pin_versionstamp = [2; 16];

		seed_bucket_branch_record(&db, source_branch_id, 0).await?;
		db.run(move |tx| async move {
			tx.informal().atomic_op(
				&bucket_branches_restore_point_pin_key(source_branch_id),
				&pin_versionstamp,
				MutationType::ByteMin,
			);
			Ok(())
		})
		.await?;

		let err = db
			.run(move |tx| async move {
				branch::derive_bucket_branch_at(
					&tx,
					source_branch_id,
					at_versionstamp,
					new_branch_id,
					None,
				)
				.await
			})
			.await
			.expect_err("bucket fork should be outside retention");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::ForkOutOfRetention
				))
		);
		assert!(
			read_value(&db, bucket_branches_list_key(new_branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn derive_bucket_branch_at_enforces_max_bucket_depth() -> Result<()> {
	udb_matrix!("depot-branch-bucket-max-depth", |ctx, db| {
		let source_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0xbbbb));
		let new_branch_id = BucketBranchId::from_uuid(uuid::Uuid::from_u128(0xcccc));

		seed_bucket_branch_record(&db, source_branch_id, depot::constants::MAX_BUCKET_DEPTH)
			.await?;

		let err = db
			.run(move |tx| async move {
				branch::derive_bucket_branch_at(&tx, source_branch_id, [1; 16], new_branch_id, None)
					.await
			})
			.await
			.expect_err("bucket fork depth should be capped");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::BucketForkChainTooDeep
				))
		);
		assert!(
			read_value(&db, bucket_branches_list_key(new_branch_id))
				.await?
				.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn fork_bucket_writes_pointer_and_metadata_without_eager_aptr() -> Result<()> {
	branch_matrix!("depot-branch-fork-bucket", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let source_bucket_id = BucketId::from_gas_id(test_bucket());
		let source_bucket_branch = read_bucket_branch_id(&db).await?;
		let source_database_branch = read_branch_id(&db).await?;
		let source_commit_bytes = read_value(&db, branch_commit_key(source_database_branch, 1))
			.await?
			.expect("source commit row should exist");
		let source_commit = decode_commit_row(&source_commit_bytes)?;

		let forked_bucket_id = branch::fork_bucket(
			&db,
			source_bucket_id,
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				restore_point: None,
			},
		)
		.await?;

		assert_ne!(forked_bucket_id, source_bucket_id);
		let pointer_bytes = read_value(&db, bucket_pointer_cur_key(forked_bucket_id))
			.await?
			.expect("forked bucket pointer should exist");
		let forked_pointer = decode_bucket_pointer(&pointer_bytes)?;
		let forked_branch_record_bytes =
			read_value(&db, bucket_branches_list_key(forked_pointer.current_branch))
				.await?
				.expect("forked bucket branch record should exist");
		let forked_record = decode_bucket_branch_record(&forked_branch_record_bytes)?;
		assert_eq!(forked_record.parent, Some(source_bucket_branch));
		assert_eq!(
			forked_record.parent_versionstamp,
			Some(source_commit.versionstamp)
		);
		assert_eq!(forked_record.fork_depth, 1);
		assert_eq!(forked_record.state, BranchState::Live);
		assert_eq!(read_bucket_refcount(&db, source_bucket_branch).await?, 2);
		assert_eq!(
			read_bucket_refcount(&db, forked_pointer.current_branch).await?,
			1
		);
		assert_eq!(
			read_bucket_pin(&db, source_bucket_branch).await?,
			source_commit.versionstamp
		);
		assert!(
			read_value(
				&db,
				database_pointer_cur_key(forked_pointer.current_branch, TEST_DATABASE)
			)
			.await?
			.is_none()
		);

		Ok(())
	})
}

#[tokio::test]
async fn fork_bucket_enforces_max_bucket_depth() -> Result<()> {
	branch_matrix!("depot-branch-fork-bucket-depth", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let source_database_branch = read_branch_id(&db).await?;
		let source_commit_bytes = read_value(&db, branch_commit_key(source_database_branch, 1))
			.await?
			.expect("source commit row should exist");
		let source_commit = decode_commit_row(&source_commit_bytes)?;
		let at = ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			restore_point: None,
		};
		let mut source_bucket_id = BucketId::from_gas_id(test_bucket());

		for _ in 0..depot::constants::MAX_BUCKET_DEPTH {
			source_bucket_id = branch::fork_bucket(&db, source_bucket_id, at.clone()).await?;
		}

		let err = branch::fork_bucket(&db, source_bucket_id, at)
			.await
			.expect_err("bucket fork depth should be capped");

		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::BucketForkChainTooDeep
				))
		);

		Ok(())
	})
}

#[tokio::test]
async fn resolve_database_pointer_walks_bucket_parent_chain_after_fork() -> Result<()> {
	branch_matrix!(
		"depot-branch-resolve-bucket-parent",
		|ctx, db, database_db| {
			database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let source_database_branch = read_branch_id(&db).await?;
			let source_commit_bytes = read_value(&db, branch_commit_key(source_database_branch, 1))
				.await?
				.expect("source commit row should exist");
			let source_commit = decode_commit_row(&source_commit_bytes)?;

			let forked_bucket_id = branch::fork_bucket(
				&db,
				BucketId::from_gas_id(test_bucket()),
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
			)
			.await?;
			let pointer_bytes = read_value(&db, bucket_pointer_cur_key(forked_bucket_id))
				.await?
				.expect("forked bucket pointer should exist");
			let forked_pointer = decode_bucket_pointer(&pointer_bytes)?;

			let resolved_pointer = db
				.run(move |tx| async move {
					branch::resolve_database_pointer(
						&tx,
						forked_pointer.current_branch,
						TEST_DATABASE,
						Snapshot,
					)
					.await
				})
				.await?
				.expect("database pointer should be inherited from parent bucket branch");
			assert_eq!(resolved_pointer.current_branch, source_database_branch);

			let forked_database_db =
				ctx.make_db(Id::v1(forked_bucket_id.as_uuid(), 1), TEST_DATABASE);
			let pages = forked_database_db.get_pages(vec![1]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn resolve_database_pointer_honors_bucket_branch_tombstones() -> Result<()> {
	branch_matrix!("depot-branch-resolve-tombstone", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let source_database_branch = read_branch_id(&db).await?;
		let source_commit_bytes = read_value(&db, branch_commit_key(source_database_branch, 1))
			.await?
			.expect("source commit row should exist");
		let source_commit = decode_commit_row(&source_commit_bytes)?;

		let forked_bucket_id = branch::fork_bucket(
			&db,
			BucketId::from_gas_id(test_bucket()),
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				restore_point: None,
			},
		)
		.await?;
		let pointer_bytes = read_value(&db, bucket_pointer_cur_key(forked_bucket_id))
			.await?
			.expect("forked bucket pointer should exist");
		let forked_pointer = decode_bucket_pointer(&pointer_bytes)?;
		let forked_bucket_branch = forked_pointer.current_branch;

		db.run(move |tx| async move {
			tx.informal().set(
				&bucket_branches_database_name_tombstone_key(forked_bucket_branch, TEST_DATABASE),
				&[],
			);
			Ok(())
		})
		.await?;

		let err = db
			.run(move |tx| async move {
				branch::resolve_database_pointer(&tx, forked_bucket_branch, TEST_DATABASE, Snapshot)
					.await
			})
			.await
			.expect_err("tombstone should block inherited database pointer");
		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::DatabaseNotFound
				))
		);

		let forked_database_db = ctx.make_db(Id::v1(forked_bucket_id.as_uuid(), 1), TEST_DATABASE);
		let err = forked_database_db
			.get_pages(vec![1])
			.await
			.expect_err("tombstoned database should not fall back to legacy storage");
		assert!(
			err.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| matches!(
					err,
					depot::error::SqliteStorageError::DatabaseNotFound
				))
		);

		Ok(())
	})
}

#[tokio::test]
async fn fork_database_writes_target_pointer_and_reads_source_data() -> Result<()> {
	branch_matrix!(
		"depot-branch-fork-database",
		|ctx, db, source_database_db| {
			source_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let source_branch_id = read_branch_id(&db).await?;
			let source_commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
				.await?
				.expect("source commit row should exist");
			let source_commit = decode_commit_row(&source_commit_bytes)?;

			let target_seed = ctx.make_db(target_bucket(), "target-seed");
			target_seed.commit(vec![page(1, 0xaa)], 1, 1_100).await?;
			let target_bucket_branch = read_bucket_branch_id_for(&db, target_bucket()).await?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(target_bucket()),
			)
			.await?;

			assert_ne!(forked_database_id, TEST_DATABASE);
			let forked_branch_id =
				read_database_branch_id(&db, target_bucket(), &forked_database_id).await?;
			let forked_record_bytes = read_value(&db, branches_list_key(forked_branch_id))
				.await?
				.expect("forked database branch record should exist");
			let forked_record = decode_database_branch_record(&forked_record_bytes)?;
			assert_eq!(forked_record.bucket_branch, target_bucket_branch);
			assert_eq!(forked_record.parent, Some(source_branch_id));
			assert_eq!(
				forked_record.parent_versionstamp,
				Some(source_commit.versionstamp)
			);
			assert_eq!(read_refcount(&db, source_branch_id).await?, 2);
			assert_eq!(read_refcount(&db, forked_branch_id).await?, 1);

			let forked_database_db = ctx.make_db(target_bucket(), forked_database_id);
			let pages = forked_database_db.get_pages(vec![1]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn fresh_fork_uses_head_at_fork_until_first_commit() -> Result<()> {
	branch_matrix!(
		"depot-branch-fresh-fork-head",
		|ctx, db, source_database_db| {
			source_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let source_branch_id = read_branch_id(&db).await?;
			let source_commit = decode_commit_row(
				&read_value(&db, branch_commit_key(source_branch_id, 1))
					.await?
					.expect("source commit row should exist"),
			)?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let forked_branch_id =
				read_database_branch_id(&db, test_bucket(), &forked_database_id).await?;
			assert!(
				read_value(&db, branch_meta_head_key(forked_branch_id))
					.await?
					.is_none()
			);
			let head_at_fork = decode_db_head(
				&read_value(&db, branch_meta_head_at_fork_key(forked_branch_id))
					.await?
					.expect("forked branch head_at_fork should exist"),
			)?;
			assert_eq!(head_at_fork.head_txid, 1);
			assert_eq!(head_at_fork.db_size_pages, 2);

			let forked_database_db = ctx.make_db(test_bucket(), forked_database_id);
			let pages = forked_database_db.get_pages(vec![1]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

			forked_database_db
				.commit(vec![page(2, 0x22)], 3, 2_000)
				.await?;
			assert!(
				read_value(&db, branch_meta_head_at_fork_key(forked_branch_id))
					.await?
					.is_none()
			);
			let forked_head = read_branch_head(&db, forked_branch_id).await?;
			assert_eq!(forked_head.head_txid, 2);
			assert_eq!(forked_head.db_size_pages, 3);
			assert!(
				read_value(&db, branch_commit_key(forked_branch_id, 2))
					.await?
					.is_some()
			);

			let pages = forked_database_db.get_pages(vec![1, 2]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn fork_database_reads_parent_shard_when_parent_pidx_is_newer_than_fork() -> Result<()> {
	branch_matrix!(
		"depot-branch-parent-shard-newer-pidx",
		|ctx, db, source_database_db| {
			source_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let source_branch_id = read_branch_id(&db).await?;
			let source_commit_bytes = read_value(&db, branch_commit_key(source_branch_id, 1))
				.await?
				.expect("source commit row should exist");
			let source_commit = decode_commit_row(&source_commit_bytes)?;

			db.run(move |tx| async move {
				tx.informal().set(
					&branch_shard_key(source_branch_id, 0, 1),
					&encoded_blob(1, vec![page(1, 0x11)])?,
				);
				Ok(())
			})
			.await?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;

			source_database_db
				.commit(vec![page(1, 0x22)], 2, 2_000)
				.await?;

			let forked_database_db = ctx.make_db(test_bucket(), forked_database_id);
			let pages = forked_database_db.get_pages(vec![1]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn fork_database_can_use_depth_one_source_branch() -> Result<()> {
	branch_matrix!(
		"depot-branch-depth-one-source",
		|ctx, db, root_database_db| {
			root_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let root_branch_id = read_branch_id(&db).await?;
			let root_commit_bytes = read_value(&db, branch_commit_key(root_branch_id, 1))
				.await?
				.expect("root commit row should exist");
			let root_commit = decode_commit_row(&root_commit_bytes)?;

			let depth_one_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: root_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let depth_one_database_db = ctx.make_db(test_bucket(), depth_one_database_id.clone());
			depth_one_database_db
				.commit(vec![page(2, 0x22)], 3, 2_000)
				.await?;
			let depth_one_branch_id =
				read_database_branch_id(&db, test_bucket(), &depth_one_database_id).await?;
			let depth_one_commit = read_head_commit(&db, depth_one_branch_id).await?;

			let depth_two_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				depth_one_database_id,
				ResolvedVersionstamp {
					versionstamp: depth_one_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let depth_two_branch_id =
				read_database_branch_id(&db, test_bucket(), &depth_two_database_id).await?;
			let depth_two_record_bytes = read_value(&db, branches_list_key(depth_two_branch_id))
				.await?
				.expect("depth-two branch record should exist");
			let depth_two_record = decode_database_branch_record(&depth_two_record_bytes)?;
			assert_eq!(depth_two_record.parent, Some(depth_one_branch_id));
			assert_eq!(depth_two_record.fork_depth, 2);

			let depth_two_database_db = ctx.make_db(test_bucket(), depth_two_database_id);
			let pages = depth_two_database_db.get_pages(vec![1, 2]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn database_db_reuses_cached_branch_ancestry_for_reads() -> Result<()> {
	branch_matrix!(
		"depot-branch-cached-ancestry",
		|ctx, db, root_database_db| {
			root_database_db
				.commit(vec![page(1, 0x11)], 2, 1_000)
				.await?;
			let root_branch_id = read_branch_id(&db).await?;
			let root_commit = decode_commit_row(
				&read_value(&db, branch_commit_key(root_branch_id, 1))
					.await?
					.expect("root commit row should exist"),
			)?;

			let child_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				TEST_DATABASE.to_string(),
				ResolvedVersionstamp {
					versionstamp: root_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let child_database_db = ctx.make_db(test_bucket(), child_database_id.clone());
			child_database_db
				.commit(vec![page(2, 0x22)], 3, 2_000)
				.await?;
			let child_branch_id =
				read_database_branch_id(&db, test_bucket(), &child_database_id).await?;
			let child_commit = read_head_commit(&db, child_branch_id).await?;

			let grandchild_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				child_database_id,
				ResolvedVersionstamp {
					versionstamp: child_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let grandchild_branch_id =
				read_database_branch_id(&db, test_bucket(), &grandchild_database_id).await?;
			let grandchild_database_db = ctx.make_db(test_bucket(), grandchild_database_id);

			let pages = grandchild_database_db.get_pages(vec![1, 2]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

			clear_value(&db, branches_list_key(grandchild_branch_id)).await?;
			clear_value(&db, branches_list_key(child_branch_id)).await?;
			clear_value(&db, branches_list_key(root_branch_id)).await?;

			let pages = grandchild_database_db.get_pages(vec![1, 2]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));

			Ok(())
		}
	)
}

#[tokio::test]
async fn fork_database_can_use_deep_source_branch() -> Result<()> {
	branch_matrix!("depot-branch-deep-source", |ctx, db, root_database_db| {
		root_database_db
			.commit(vec![page(1, 0x11)], 2, 1_000)
			.await?;
		let mut source_database_id = TEST_DATABASE.to_string();
		let mut source_branch_id = read_branch_id(&db).await?;
		let mut source_commit = decode_commit_row(
			&read_value(&db, branch_commit_key(source_branch_id, 1))
				.await?
				.expect("root commit row should exist"),
		)?;

		for depth in 1..=3 {
			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(test_bucket()),
				source_database_id,
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(test_bucket()),
			)
			.await?;
			let forked_database_db = ctx.make_db(test_bucket(), forked_database_id.clone());
			forked_database_db
				.commit(
					vec![page(depth + 1, 0x20 + depth as u8)],
					depth + 2,
					2_000 + depth as i64,
				)
				.await?;
			source_database_id = forked_database_id;
			source_branch_id =
				read_database_branch_id(&db, test_bucket(), &source_database_id).await?;
			source_commit = read_head_commit(&db, source_branch_id).await?;
		}

		let final_database_id = branch::fork_database(
			&db,
			BucketId::from_gas_id(test_bucket()),
			source_database_id,
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				restore_point: None,
			},
			BucketId::from_gas_id(test_bucket()),
		)
		.await?;
		let final_branch_id =
			read_database_branch_id(&db, test_bucket(), &final_database_id).await?;
		let final_record_bytes = read_value(&db, branches_list_key(final_branch_id))
			.await?
			.expect("final branch record should exist");
		let final_record = decode_database_branch_record(&final_record_bytes)?;
		assert_eq!(final_record.fork_depth, 4);

		let final_database_db = ctx.make_db(test_bucket(), final_database_id);
		let pages = final_database_db.get_pages(vec![1, 2, 3, 4]).await?;
		assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
		assert_eq!(pages[1].bytes, Some(page_bytes(0x21)));
		assert_eq!(pages[2].bytes, Some(page_bytes(0x22)));
		assert_eq!(pages[3].bytes, Some(page_bytes(0x23)));

		Ok(())
	})
}

#[tokio::test]
async fn rollback_database_freezes_old_branch_and_swaps_pointer() -> Result<()> {
	branch_matrix!("depot-branch-rollback-database", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		database_db.commit(vec![page(1, 0x22)], 2, 2_000).await?;
		let old_branch_id = read_branch_id(&db).await?;
		let bucket_branch_id = read_bucket_branch_id(&db).await?;
		let first_commit = decode_commit_row(
			&read_value(&db, branch_commit_key(old_branch_id, 1))
				.await?
				.expect("first commit row should exist"),
		)?;
		let pages = database_db.get_pages(vec![1]).await?;
		assert_eq!(pages[0].bytes, Some(page_bytes(0x22)));

		let rolled_branch_id = branch::rollback_database(
			&db,
			BucketId::from_gas_id(test_bucket()),
			TEST_DATABASE.to_string(),
			ResolvedVersionstamp {
				versionstamp: first_commit.versionstamp,
				restore_point: None,
			},
		)
		.await?;

		assert_ne!(rolled_branch_id, old_branch_id);
		let current_branch_id = read_branch_id(&db).await?;
		assert_eq!(current_branch_id, rolled_branch_id);

		let old_record_bytes = read_value(&db, branches_list_key(old_branch_id))
			.await?
			.expect("old branch record should exist");
		let old_record = decode_database_branch_record(&old_record_bytes)?;
		assert_eq!(old_record.state, BranchState::Frozen);
		assert_eq!(read_refcount(&db, old_branch_id).await?, 1);
		assert_eq!(read_refcount(&db, rolled_branch_id).await?, 1);

		let rolled_record_bytes = read_value(&db, branches_list_key(rolled_branch_id))
			.await?
			.expect("rolled branch record should exist");
		let rolled_record = decode_database_branch_record(&rolled_record_bytes)?;
		assert_eq!(rolled_record.parent, Some(old_branch_id));
		assert_eq!(
			rolled_record.parent_versionstamp,
			Some(first_commit.versionstamp)
		);
		assert_eq!(rolled_record.state, BranchState::Live);

		let history_values = read_prefix_values(
			&db,
			database_pointer_history_prefix(bucket_branch_id, TEST_DATABASE),
		)
		.await?;
		assert_eq!(history_values.len(), 1);
		let history_pointer = decode_database_pointer(&history_values[0])?;
		assert_eq!(history_pointer.current_branch, old_branch_id);

		database_db.commit(vec![page(2, 0x33)], 3, 3_000).await?;

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
	})
}

#[tokio::test]
async fn rollback_bucket_freezes_old_branch_and_swaps_pointer() -> Result<()> {
	branch_matrix!("depot-branch-rollback-bucket", |ctx, db, database_db| {
		database_db.commit(vec![page(1, 0x11)], 2, 1_000).await?;
		let old_bucket_branch_id = read_bucket_branch_id(&db).await?;
		let database_branch_id = read_branch_id(&db).await?;
		let first_commit = decode_commit_row(
			&read_value(&db, branch_commit_key(database_branch_id, 1))
				.await?
				.expect("first commit row should exist"),
		)?;

		let rolled_bucket_branch_id = branch::rollback_bucket(
			&db,
			BucketId::from_gas_id(test_bucket()),
			ResolvedVersionstamp {
				versionstamp: first_commit.versionstamp,
				restore_point: None,
			},
		)
		.await?;

		assert_ne!(rolled_bucket_branch_id, old_bucket_branch_id);
		let current_bucket_branch_id = read_bucket_branch_id(&db).await?;
		assert_eq!(current_bucket_branch_id, rolled_bucket_branch_id);

		let old_record_bytes = read_value(&db, bucket_branches_list_key(old_bucket_branch_id))
			.await?
			.expect("old bucket branch record should exist");
		let old_record = decode_bucket_branch_record(&old_record_bytes)?;
		assert_eq!(old_record.state, BranchState::Frozen);
		assert_eq!(read_bucket_refcount(&db, old_bucket_branch_id).await?, 1);
		assert_eq!(read_bucket_refcount(&db, rolled_bucket_branch_id).await?, 1);

		let rolled_record_bytes =
			read_value(&db, bucket_branches_list_key(rolled_bucket_branch_id))
				.await?
				.expect("rolled bucket branch record should exist");
		let rolled_record = decode_bucket_branch_record(&rolled_record_bytes)?;
		assert_eq!(rolled_record.parent, Some(old_bucket_branch_id));
		assert_eq!(
			rolled_record.parent_versionstamp,
			Some(first_commit.versionstamp)
		);
		assert_eq!(rolled_record.state, BranchState::Live);

		let history_values = read_prefix_values(
			&db,
			bucket_pointer_history_prefix(BucketId::from_gas_id(test_bucket())),
		)
		.await?;
		assert_eq!(history_values.len(), 1);
		let history_pointer = decode_bucket_pointer(&history_values[0])?;
		assert_eq!(history_pointer.current_branch, old_bucket_branch_id);

		let rolled_database_db = ctx.make_db(test_bucket(), TEST_DATABASE);
		let pages = rolled_database_db.get_pages(vec![1]).await?;
		assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

		Ok(())
	})
}

async fn seed_branch_record(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	bucket_branch: BucketBranchId,
	fork_depth: u8,
) -> Result<()> {
	let record = DatabaseBranchRecord {
		branch_id,
		bucket_branch,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [0; 16],
		fork_depth,
		created_at_ms: 1_000,
		created_from_restore_point: None,
		state: BranchState::Live,
		lifecycle_generation: 0,
	};
	let encoded_record = encode_database_branch_record(record)?;
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

async fn seed_bucket_branch_record(
	db: &universaldb::Database,
	branch_id: BucketBranchId,
	fork_depth: u8,
) -> Result<()> {
	let record = BucketBranchRecord {
		branch_id,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [0; 16],
		fork_depth,
		created_at_ms: 1_000,
		created_from_restore_point: None,
		state: BranchState::Live,
	};
	let encoded_record = encode_bucket_branch_record(record)?;
	db.run(move |tx| {
		let encoded_record = encoded_record.clone();
		async move {
			tx.informal()
				.set(&bucket_branches_list_key(branch_id), &encoded_record);
			tx.informal().atomic_op(
				&bucket_branches_refcount_key(branch_id),
				&1_i64.to_le_bytes(),
				MutationType::Add,
			);
			Ok(())
		}
	})
	.await
}
