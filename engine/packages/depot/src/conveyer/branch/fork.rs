use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Serializable};

use super::{
	catalog::{write_bucket_catalog_marker, write_bucket_fork_facts},
	resolve::{resolve_bucket_branch, resolve_database_branch_in_bucket},
	shared::{
		lookup_txid_at_versionstamp, now_ms, read_bucket_branch_record, read_commit_row,
		read_database_branch_record, read_versionstamp_pin,
	},
};
use crate::conveyer::{
	constants::{MAX_BUCKET_DEPTH, MAX_FORK_DEPTH},
	error::SqliteStorageError,
	history_pin, keys, restore_point,
	types::{
		BranchState, BucketBranchId, BucketBranchRecord, BucketId, BucketPointer, DBHead,
		DatabaseBranchId, DatabaseBranchRecord, DatabasePointer, ResolvedRestoreTarget,
		ResolvedVersionstamp, RestorePointRef, SnapshotSelector, encode_bucket_branch_record,
		encode_bucket_pointer, encode_database_branch_record, encode_database_pointer,
		encode_db_head,
	},
	udb,
};
use crate::gc;

#[derive(Debug, Clone)]
pub enum DatabaseForkTarget {
	Resolved(ResolvedVersionstamp),
	Selector(SnapshotSelector),
}

impl From<ResolvedVersionstamp> for DatabaseForkTarget {
	fn from(value: ResolvedVersionstamp) -> Self {
		Self::Resolved(value)
	}
}

impl From<SnapshotSelector> for DatabaseForkTarget {
	fn from(value: SnapshotSelector) -> Self {
		Self::Selector(value)
	}
}

pub async fn fork_database<T>(
	udb: &universaldb::Database,
	source_bucket: BucketId,
	source_database_id: String,
	target: T,
	target_bucket: BucketId,
) -> Result<String>
where
	T: Into<DatabaseForkTarget>,
{
	let new_database_id = format!("fork-{}", uuid::Uuid::new_v4().simple());
	let new_database_branch_id = DatabaseBranchId::new_v4();
	let target = match target.into() {
		DatabaseForkTarget::Resolved(at) => ResolvedForkTarget::CurrentSourceBranch(at),
		DatabaseForkTarget::Selector(selector) => {
			let target = restore_point::resolve_restore_target(
				udb,
				source_bucket,
				source_database_id.clone(),
				selector,
			)
			.await?;
			ResolvedForkTarget::ResolvedTarget(target)
		}
	};
	let target_for_tx = target.clone();

	udb.run({
		let new_database_id = new_database_id.clone();
		move |tx| {
			let source_database_id = source_database_id.clone();
			let new_database_id = new_database_id.clone();
			let target = target_for_tx.clone();

			async move {
				let source_bucket_branch = resolve_bucket_branch(&tx, source_bucket, Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;
				let target_bucket_branch = resolve_bucket_branch(&tx, target_bucket, Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;
				let (source_database_branch, at_versionstamp, restore_point) = match target {
					ResolvedForkTarget::CurrentSourceBranch(at) => {
						let source_database_branch = resolve_database_branch_in_bucket(
							&tx,
							source_bucket_branch,
							&source_database_id,
							Serializable,
						)
						.await?
						.ok_or(SqliteStorageError::DatabaseNotFound)?;
						(source_database_branch, at.versionstamp, at.restore_point)
					}
					ResolvedForkTarget::ResolvedTarget(target) => (
						target.database_branch_id,
						target.versionstamp,
						target.restore_point,
					),
				};

				derive_branch_at(
					&tx,
					source_database_branch,
					at_versionstamp,
					new_database_branch_id,
					target_bucket_branch,
					restore_point,
				)
				.await?;

				let pointer = DatabasePointer {
					current_branch: new_database_branch_id,
					last_swapped_at_ms: now_ms()?,
				};
				let encoded_pointer = encode_database_pointer(pointer)
					.context("encode sqlite fork database pointer")?;
				tx.informal().set(
					&keys::database_pointer_cur_key(target_bucket_branch, &new_database_id),
					&encoded_pointer,
				);
				write_bucket_catalog_marker(
					&tx,
					target_bucket_branch,
					new_database_branch_id,
					&udb::INCOMPLETE_VERSIONSTAMP,
				)
				.await?;

				Ok(())
			}
		}
	})
	.await?;

	Ok(new_database_id)
}

#[derive(Debug, Clone)]
enum ResolvedForkTarget {
	CurrentSourceBranch(ResolvedVersionstamp),
	ResolvedTarget(ResolvedRestoreTarget),
}

pub async fn fork_bucket(
	udb: &universaldb::Database,
	source_bucket: BucketId,
	at: ResolvedVersionstamp,
) -> Result<BucketId> {
	let new_bucket_id = BucketId::new_v4();
	let new_bucket_branch_id = BucketBranchId::new_v4();
	let at_for_tx = at.clone();

	udb.run({
		move |tx| {
			let at = at_for_tx.clone();

			async move {
				let source_bucket_branch = resolve_bucket_branch(&tx, source_bucket, Serializable)
					.await?
					.ok_or(SqliteStorageError::DatabaseNotFound)?;

				derive_bucket_branch_at(
					&tx,
					source_bucket_branch,
					at.versionstamp,
					new_bucket_branch_id,
					at.restore_point,
				)
				.await?;

				let pointer = BucketPointer {
					current_branch: new_bucket_branch_id,
					last_swapped_at_ms: now_ms()?,
				};
				let encoded_pointer =
					encode_bucket_pointer(pointer).context("encode sqlite fork bucket pointer")?;
				tx.informal().set(
					&keys::bucket_pointer_cur_key(new_bucket_id),
					&encoded_pointer,
				);

				Ok(())
			}
		}
	})
	.await?;

	Ok(new_bucket_id)
}

pub async fn derive_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: DatabaseBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: DatabaseBranchId,
	bucket_branch: BucketBranchId,
	restore_point_ref: Option<RestorePointRef>,
) -> Result<()> {
	let source = read_database_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_FORK_DEPTH {
		return Err(SqliteStorageError::ForkChainTooDeep.into());
	}

	let restore_point_pin =
		read_versionstamp_pin(tx, &keys::branches_restore_point_pin_key(source_branch_id)).await?;
	if restore_point_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let source_gc_pin = gc::read_branch_gc_pin_tx(tx, source_branch_id)
		.await?
		.context("sqlite source branch GC pin is missing")?;
	if source_gc_pin.gc_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let txid_at_versionstamp = lookup_txid_at_versionstamp(tx, source_branch_id, at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"lookup sqlite VTX entry for database branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let commit_at_versionstamp = read_commit_row(tx, source_branch_id, txid_at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"read sqlite commit row {txid_at_versionstamp} for database branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let now_ms = now_ms()?;
	let head_at_fork = DBHead {
		head_txid: txid_at_versionstamp,
		db_size_pages: commit_at_versionstamp.db_size_pages,
		post_apply_checksum: commit_at_versionstamp.post_apply_checksum,
		branch_id: new_branch_id,
		#[cfg(debug_assertions)]
		generation: 0,
	};
	let encoded_head_at_fork =
		encode_db_head(head_at_fork).context("encode sqlite fork head snapshot")?;
	tx.informal().set(
		&keys::branch_meta_head_at_fork_key(new_branch_id),
		&encoded_head_at_fork,
	);

	let new_record = DatabaseBranchRecord {
		branch_id: new_branch_id,
		bucket_branch,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms,
		created_from_restore_point: restore_point_ref,
		state: BranchState::Live,
		lifecycle_generation: 0,
	};
	let encoded_record = encode_database_branch_record(new_record)
		.context("encode sqlite derived database branch record")?;
	tx.informal()
		.set(&keys::branches_list_key(new_branch_id), &encoded_record);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);
	history_pin::write_database_fork_pin(
		tx,
		source_branch_id,
		new_branch_id,
		at_versionstamp,
		txid_at_versionstamp,
		now_ms,
	)?;

	Ok(())
}

pub async fn derive_bucket_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: BucketBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: BucketBranchId,
	restore_point_ref: Option<RestorePointRef>,
) -> Result<()> {
	let source = read_bucket_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_BUCKET_DEPTH {
		return Err(SqliteStorageError::BucketForkChainTooDeep.into());
	}

	let restore_point_pin = read_versionstamp_pin(
		tx,
		&keys::bucket_branches_restore_point_pin_key(source_branch_id),
	)
	.await?;
	if restore_point_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let new_record = BucketBranchRecord {
		branch_id: new_branch_id,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms()?,
		created_from_restore_point: restore_point_ref,
		state: BranchState::Live,
	};
	let encoded_record = encode_bucket_branch_record(new_record)
		.context("encode sqlite derived bucket branch record")?;
	tx.informal().set(
		&keys::bucket_branches_list_key(new_branch_id),
		&encoded_record,
	);
	tx.informal().atomic_op(
		&keys::bucket_branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::bucket_branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::bucket_branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);
	write_bucket_fork_facts(tx, source_branch_id, new_branch_id, at_versionstamp).await?;

	Ok(())
}
