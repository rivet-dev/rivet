use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Serializable};

use super::{
	catalog::{
		is_database_visible_in_bucket_branch, versionstamped_marker_value,
		write_bucket_catalog_tombstone_marker,
	},
	fork::{derive_branch_at, derive_bucket_branch_at},
	resolve::{resolve_bucket_branch, resolve_database_pointer},
	shared::{now_ms, read_bucket_branch_record, read_database_branch_record},
};
use crate::conveyer::{
	error::SqliteStorageError,
	keys,
	types::{
		BranchState, BucketBranchId, BucketBranchRecord, BucketId, BucketPointer, DatabaseBranchId,
		DatabaseBranchRecord, DatabasePointer, ResolvedRestoreTarget, ResolvedVersionstamp,
		decode_bucket_pointer, encode_bucket_branch_record, encode_bucket_pointer,
		encode_database_branch_record, encode_database_pointer,
	},
};

pub async fn delete_database(
	udb: &universaldb::Database,
	bucket: BucketId,
	database_id: DatabaseBranchId,
) -> Result<()> {
	udb.run(move |tx| async move {
		let bucket_branch_id = resolve_bucket_branch(&tx, bucket, Serializable)
			.await?
			.ok_or(SqliteStorageError::DatabaseNotFound)?;

		let visible =
			is_database_visible_in_bucket_branch(&tx, bucket_branch_id, database_id).await?;
		if !visible {
			return Err(SqliteStorageError::DatabaseNotFound.into());
		}

		tx.informal().atomic_op(
			&keys::bucket_branches_database_tombstone_key(bucket_branch_id, database_id),
			&versionstamped_marker_value()
				.context("prepare versionstamped sqlite bucket database tombstone")?,
			MutationType::SetVersionstampedValue,
		);
		write_bucket_catalog_tombstone_marker(&tx, bucket_branch_id, database_id).await?;
		tx.informal().atomic_op(
			&keys::branches_refcount_key(database_id),
			&(-1_i64).to_le_bytes(),
			MutationType::Add,
		);

		Ok(())
	})
	.await
}

pub async fn rollback_bucket(
	udb: &universaldb::Database,
	bucket: BucketId,
	at: ResolvedVersionstamp,
) -> Result<BucketBranchId> {
	let rolled_branch_id = BucketBranchId::new_v4();

	udb.run({
		let at = at.clone();

		move |tx| {
			let at = at.clone();

			async move {
				let cur_ptr_bytes = tx
					.informal()
					.get(&keys::bucket_pointer_cur_key(bucket), Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;
				let cur_ptr = decode_bucket_pointer(&cur_ptr_bytes)
					.context("decode sqlite bucket pointer for rollback")?;
				let cur_record = read_bucket_branch_record(&tx, cur_ptr.current_branch).await?;

				derive_bucket_branch_at(
					&tx,
					cur_ptr.current_branch,
					at.versionstamp,
					rolled_branch_id,
					at.restore_point,
				)
				.await?;
				freeze_bucket_branch(&tx, cur_record).await?;

				let now_ms = now_ms()?;
				let nonce = uuid::Uuid::new_v4().as_u128() as u32;
				let encoded_history_pointer = encode_bucket_pointer(cur_ptr.clone())
					.context("encode sqlite rollback bucket pointer history")?;
				tx.informal().set(
					&keys::bucket_pointer_history_key(bucket, now_ms, nonce),
					&encoded_history_pointer,
				);
				tx.informal().atomic_op(
					&keys::bucket_branches_refcount_key(cur_ptr.current_branch),
					&(-1_i64).to_le_bytes(),
					MutationType::Add,
				);

				let new_ptr = BucketPointer {
					current_branch: rolled_branch_id,
					last_swapped_at_ms: now_ms,
				};
				let encoded_pointer = encode_bucket_pointer(new_ptr)
					.context("encode sqlite rollback bucket pointer")?;
				tx.informal()
					.set(&keys::bucket_pointer_cur_key(bucket), &encoded_pointer);

				Ok(())
			}
		}
	})
	.await?;

	Ok(rolled_branch_id)
}

pub async fn rollback_database(
	udb: &universaldb::Database,
	bucket: BucketId,
	database_id: String,
	at: ResolvedVersionstamp,
) -> Result<DatabaseBranchId> {
	let rolled_branch_id = DatabaseBranchId::new_v4();

	udb.run({
		let database_id = database_id.clone();
		let at = at.clone();

		move |tx| {
			let database_id = database_id.clone();
			let at = at.clone();

			async move {
				let bucket_branch = resolve_bucket_branch(&tx, bucket, Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;
				let cur_ptr =
					resolve_database_pointer(&tx, bucket_branch, &database_id, Serializable)
						.await?
						.ok_or(SqliteStorageError::DatabaseNotFound)?;
				let cur_record = read_database_branch_record(&tx, cur_ptr.current_branch).await?;

				derive_branch_at(
					&tx,
					cur_ptr.current_branch,
					at.versionstamp,
					rolled_branch_id,
					cur_record.bucket_branch,
					at.restore_point,
				)
				.await?;
				freeze_database_branch(&tx, cur_record).await?;

				let now_ms = now_ms()?;
				let nonce = uuid::Uuid::new_v4().as_u128() as u32;
				let encoded_history_pointer = encode_database_pointer(cur_ptr.clone())
					.context("encode sqlite rollback database pointer history")?;
				tx.informal().set(
					&keys::database_pointer_history_key(bucket_branch, &database_id, now_ms, nonce),
					&encoded_history_pointer,
				);
				tx.informal().atomic_op(
					&keys::branches_refcount_key(cur_ptr.current_branch),
					&(-1_i64).to_le_bytes(),
					MutationType::Add,
				);

				let new_ptr = DatabasePointer {
					current_branch: rolled_branch_id,
					last_swapped_at_ms: now_ms,
				};
				let encoded_pointer = encode_database_pointer(new_ptr)
					.context("encode sqlite rollback database pointer")?;
				tx.informal().set(
					&keys::database_pointer_cur_key(bucket_branch, &database_id),
					&encoded_pointer,
				);

				Ok(())
			}
		}
	})
	.await?;

	Ok(rolled_branch_id)
}

pub(crate) async fn rollback_database_to_target_tx(
	tx: &universaldb::Transaction,
	bucket: BucketId,
	database_id: &str,
	target: &ResolvedRestoreTarget,
	rolled_branch_id: DatabaseBranchId,
) -> Result<()> {
	let bucket_branch = resolve_bucket_branch(tx, bucket, Serializable)
		.await?
		.ok_or(SqliteStorageError::DatabaseNotFound)?;
	let cur_ptr = resolve_database_pointer(tx, bucket_branch, database_id, Serializable)
		.await?
		.ok_or(SqliteStorageError::DatabaseNotFound)?;
	let cur_record = read_database_branch_record(tx, cur_ptr.current_branch).await?;

	derive_branch_at(
		tx,
		target.database_branch_id,
		target.versionstamp,
		rolled_branch_id,
		cur_record.bucket_branch,
		target.restore_point.clone(),
	)
	.await?;
	freeze_database_branch(tx, cur_record).await?;

	let now_ms = now_ms()?;
	let nonce = uuid::Uuid::new_v4().as_u128() as u32;
	let encoded_history_pointer = encode_database_pointer(cur_ptr.clone())
		.context("encode sqlite rollback database pointer history")?;
	tx.informal().set(
		&keys::database_pointer_history_key(bucket_branch, database_id, now_ms, nonce),
		&encoded_history_pointer,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(cur_ptr.current_branch),
		&(-1_i64).to_le_bytes(),
		MutationType::Add,
	);

	let new_ptr = DatabasePointer {
		current_branch: rolled_branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer =
		encode_database_pointer(new_ptr).context("encode sqlite rollback database pointer")?;
	tx.informal().set(
		&keys::database_pointer_cur_key(bucket_branch, database_id),
		&encoded_pointer,
	);

	Ok(())
}

async fn freeze_database_branch(
	tx: &universaldb::Transaction,
	mut record: DatabaseBranchRecord,
) -> Result<()> {
	record.state = BranchState::Frozen;
	let branch_id = record.branch_id;
	let encoded_record = encode_database_branch_record(record)
		.context("encode frozen sqlite database branch record")?;
	tx.informal()
		.set(&keys::branches_list_key(branch_id), &encoded_record);

	Ok(())
}

async fn freeze_bucket_branch(
	tx: &universaldb::Transaction,
	mut record: BucketBranchRecord,
) -> Result<()> {
	record.state = BranchState::Frozen;
	let branch_id = record.branch_id;
	let encoded_record =
		encode_bucket_branch_record(record).context("encode frozen sqlite bucket branch record")?;
	tx.informal()
		.set(&keys::bucket_branches_list_key(branch_id), &encoded_record);

	Ok(())
}
