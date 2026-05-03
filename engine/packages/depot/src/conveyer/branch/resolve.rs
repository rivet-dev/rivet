use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel};

use crate::conveyer::{
	constants::MAX_BUCKET_DEPTH,
	error::SqliteStorageError,
	keys,
	types::{
		BranchState, BucketBranchId, BucketBranchRecord, BucketId, BucketPointer, DatabaseBranchId,
		DatabasePointer, decode_bucket_branch_record, decode_bucket_pointer,
		decode_database_pointer, encode_bucket_branch_record, encode_bucket_pointer,
	},
	udb,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketBranchResolution {
	pub branch_id: BucketBranchId,
	pub initialized: bool,
}

pub async fn resolve_or_allocate_root_bucket_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
) -> Result<BucketBranchResolution> {
	if let Some(branch_id) =
		resolve_bucket_branch(tx, bucket_id, IsolationLevel::Serializable).await?
	{
		return Ok(BucketBranchResolution {
			branch_id,
			initialized: false,
		});
	}

	Ok(BucketBranchResolution {
		branch_id: BucketBranchId::new_v4(),
		initialized: true,
	})
}

pub async fn resolve_bucket_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	isolation_level: IsolationLevel,
) -> Result<Option<BucketBranchId>> {
	let Some(pointer_bytes) = tx
		.informal()
		.get(&keys::bucket_pointer_cur_key(bucket_id), isolation_level)
		.await?
	else {
		return Ok(None);
	};

	let pointer = decode_bucket_pointer(&pointer_bytes).context("decode sqlite bucket pointer")?;
	Ok(Some(pointer.current_branch))
}

pub fn write_root_bucket_metadata(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	branch_id: BucketBranchId,
	now_ms: i64,
	root_versionstamp: &[u8; 16],
) -> Result<()> {
	let record = BucketBranchRecord {
		branch_id,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: *root_versionstamp,
		fork_depth: 0,
		created_at_ms: now_ms,
		created_from_restore_point: None,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_bucket_branch_record(record).context("encode sqlite root bucket branch record")?;
	let versionstamped_record = udb::append_versionstamp_offset(encoded_record, root_versionstamp)
		.context("prepare versionstamped sqlite root bucket branch record")?;
	tx.informal().atomic_op(
		&keys::bucket_branches_list_key(branch_id),
		&versionstamped_record,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&keys::bucket_branches_refcount_key(branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);

	let pointer = BucketPointer {
		current_branch: branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer = encode_bucket_pointer(pointer).context("encode sqlite bucket pointer")?;
	tx.informal()
		.set(&keys::bucket_pointer_cur_key(bucket_id), &encoded_pointer);

	Ok(())
}

pub async fn resolve_database_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<DatabaseBranchId>> {
	let Some(bucket_branch_id) = resolve_bucket_branch(tx, bucket_id, isolation_level).await?
	else {
		return resolve_database_branch_in_bucket(
			tx,
			BucketBranchId::nil(),
			database_id,
			isolation_level,
		)
		.await;
	};

	if let Some(branch_id) =
		resolve_database_branch_in_bucket(tx, bucket_branch_id, database_id, isolation_level)
			.await?
	{
		return Ok(Some(branch_id));
	}

	resolve_database_branch_in_bucket(tx, BucketBranchId::nil(), database_id, isolation_level).await
}

pub async fn resolve_database_branch_in_bucket(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<DatabaseBranchId>> {
	Ok(
		resolve_database_pointer(tx, bucket_branch_id, database_id, isolation_level)
			.await?
			.map(|pointer| pointer.current_branch),
	)
}

pub async fn resolve_database_pointer(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<DatabasePointer>> {
	let mut current_branch_id = bucket_branch_id;

	for _ in 0..=MAX_BUCKET_DEPTH {
		if let Some(pointer_bytes) = tx
			.informal()
			.get(
				&keys::database_pointer_cur_key(current_branch_id, database_id),
				isolation_level,
			)
			.await?
		{
			let pointer = decode_database_pointer(&pointer_bytes)
				.context("decode sqlite database pointer")?;
			return Ok(Some(pointer));
		}

		if current_branch_id == BucketBranchId::nil() {
			return Ok(None);
		}

		if tx
			.informal()
			.get(
				&keys::bucket_branches_database_name_tombstone_key(current_branch_id, database_id),
				isolation_level,
			)
			.await?
			.is_some()
		{
			return Err(SqliteStorageError::DatabaseNotFound.into());
		}

		let Some(record_bytes) = tx
			.informal()
			.get(
				&keys::bucket_branches_list_key(current_branch_id),
				isolation_level,
			)
			.await?
		else {
			return Ok(None);
		};
		let record = decode_bucket_branch_record(&record_bytes)
			.context("decode sqlite bucket branch record")?;
		let Some(parent) = record.parent else {
			return Ok(None);
		};
		current_branch_id = parent;
	}

	Err(SqliteStorageError::BucketForkChainTooDeep.into())
}
