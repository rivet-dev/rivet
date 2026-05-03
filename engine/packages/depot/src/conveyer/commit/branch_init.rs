use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Serializable};

use crate::conveyer::{
	branch, keys,
	types::{
		BranchState, BucketBranchId, BucketId, DatabaseBranchId, DatabaseBranchRecord,
		DatabasePointer, encode_database_branch_record, encode_database_pointer,
	},
	udb,
};

pub(super) struct BranchResolution {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) bucket_branch_id: BucketBranchId,
	pub(super) bucket_initialized: bool,
	pub(super) database_initialized: bool,
}

pub(super) async fn resolve_or_allocate_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
) -> Result<BranchResolution> {
	let bucket = branch::resolve_or_allocate_root_bucket_branch(tx, bucket_id).await?;

	if let Some(branch_id) =
		branch::resolve_database_branch_in_bucket(tx, bucket.branch_id, database_id, Serializable)
			.await?
	{
		return Ok(BranchResolution {
			branch_id,
			bucket_branch_id: bucket.branch_id,
			bucket_initialized: bucket.initialized,
			database_initialized: false,
		});
	}

	Ok(BranchResolution {
		branch_id: DatabaseBranchId::new_v4(),
		bucket_branch_id: bucket.branch_id,
		bucket_initialized: bucket.initialized,
		database_initialized: true,
	})
}

pub(super) async fn write_root_branch_metadata(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	bucket_branch: BucketBranchId,
	database_id: &str,
	now_ms: i64,
	root_versionstamp: &[u8; 16],
	bucket_initialized: bool,
) -> Result<()> {
	let record = DatabaseBranchRecord {
		branch_id,
		bucket_branch,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: *root_versionstamp,
		fork_depth: 0,
		created_at_ms: now_ms,
		created_from_restore_point: None,
		state: BranchState::Live,
		lifecycle_generation: 0,
	};
	let encoded_record = encode_database_branch_record(record)
		.context("encode sqlite root database branch record")?;
	let versionstamped_record = udb::append_versionstamp_offset(encoded_record, root_versionstamp)
		.context("prepare versionstamped sqlite root database branch record")?;
	tx.informal().atomic_op(
		&keys::branches_list_key(branch_id),
		&versionstamped_record,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	if bucket_initialized {
		branch::write_bucket_catalog_marker_with_root(
			tx,
			bucket_branch,
			bucket_branch,
			branch_id,
			root_versionstamp,
		)?;
	} else {
		branch::write_bucket_catalog_marker(tx, bucket_branch, branch_id, root_versionstamp)
			.await?;
	}

	let pointer = DatabasePointer {
		current_branch: branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer =
		encode_database_pointer(pointer).context("encode sqlite database pointer")?;
	tx.informal().set(
		&keys::database_pointer_cur_key(bucket_branch, database_id),
		&encoded_pointer,
	);

	Ok(())
}
