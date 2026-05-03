#![allow(dead_code)]

#[path = "../common/mod.rs"]
mod common;

use anyhow::Result;
use depot::{
	keys::{
		branch_commit_key, branch_meta_head_key, branches_list_key, bucket_branches_list_key,
		bucket_pointer_cur_key, database_pointer_cur_key,
	},
	types::{
		BucketBranchId, BucketBranchRecord, BucketId, CommitRow, DatabaseBranchId,
		DatabaseBranchRecord, DirtyPage, decode_bucket_branch_record, decode_bucket_pointer,
		decode_commit_row, decode_database_branch_record, decode_database_pointer, decode_db_head,
	},
};
use gas::prelude::Id;

pub use common::read_value;

pub const TEST_DATABASE: &str = "fork-source";

pub fn test_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

pub fn target_bucket() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x5678), 1)
}

pub fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; depot::keys::PAGE_SIZE as usize],
	}
}

pub fn page_bytes(fill: u8) -> Vec<u8> {
	vec![fill; depot::keys::PAGE_SIZE as usize]
}

pub async fn read_bucket_branch_id_for(
	db: &universaldb::Database,
	bucket_id: Id,
) -> Result<BucketBranchId> {
	let bucket_id = BucketId::from_gas_id(bucket_id);
	let bucket_pointer_bytes = read_value(db, bucket_pointer_cur_key(bucket_id))
		.await?
		.expect("bucket pointer should exist");

	Ok(decode_bucket_pointer(&bucket_pointer_bytes)?.current_branch)
}

pub async fn read_database_branch_id(
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

pub async fn read_database_branch_record(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
) -> Result<DatabaseBranchRecord> {
	let bytes = read_value(db, branches_list_key(branch_id))
		.await?
		.expect("database branch record should exist");

	decode_database_branch_record(&bytes)
}

pub async fn read_bucket_branch_record(
	db: &universaldb::Database,
	branch_id: BucketBranchId,
) -> Result<BucketBranchRecord> {
	let bytes = read_value(db, bucket_branches_list_key(branch_id))
		.await?
		.expect("bucket branch record should exist");

	decode_bucket_branch_record(&bytes)
}

pub async fn read_head_commit(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
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
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<CommitRow> {
	let bytes = read_value(db, branch_commit_key(branch_id, txid))
		.await?
		.expect("branch commit row should exist");

	decode_commit_row(&bytes)
}

pub fn assert_storage_error(err: &anyhow::Error, expected: depot::error::SqliteStorageError) {
	assert!(
		err.chain().any(|cause| {
			cause
				.downcast_ref::<depot::error::SqliteStorageError>()
				.is_some_and(|err| err == &expected)
		}),
		"expected {expected:?}, got {err:?}",
	);
}
