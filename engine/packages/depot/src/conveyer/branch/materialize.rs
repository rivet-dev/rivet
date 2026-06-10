//! Lazy database materialization for bucket forks.
//!
//! Bucket forks copy no per-database state: a database inherited through the
//! bucket parent chain stays owned by the source bucket until the first data
//! access through the fork. That access materializes a capped database fork at
//! the newest covered point at or below the fork chain's versionstamp cap, so
//! reads freeze at the fork point and writes build on the inherited state.

use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Serializable;

use super::{
	fork::derive_branch_at,
	resolve::{resolve_bucket_branch, resolve_database_pointer},
};
use crate::conveyer::{
	constants::MAX_BUCKET_DEPTH,
	error::SqliteStorageError,
	history_pin, keys,
	types::{
		BucketBranchId, BucketId, DatabaseBranchId, DatabasePointer, decode_bucket_branch_record,
		decode_database_pointer, encode_database_pointer,
	},
};

/// Resolves the database branch for data-plane access, materializing a capped
/// fork when the database is inherited through a bucket parent chain. Returns
/// `None` when no database exists under this name anywhere in the chain.
pub async fn resolve_or_materialize_database_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
	now_ms: i64,
	allow_materialize: bool,
) -> Result<Option<DatabaseBranchId>> {
	let Some(bucket_branch_id) = resolve_bucket_branch(tx, bucket_id, Serializable).await? else {
		return Ok(
			resolve_database_pointer(tx, BucketBranchId::nil(), database_id, Serializable)
				.await?
				.map(|pointer| pointer.current_branch),
		);
	};

	resolve_or_materialize_in_bucket_branch(
		tx,
		bucket_id,
		bucket_branch_id,
		database_id,
		now_ms,
		allow_materialize,
	)
	.await
}

pub async fn resolve_or_materialize_in_bucket_branch(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	bucket_branch_id: BucketBranchId,
	database_id: &str,
	now_ms: i64,
	allow_materialize: bool,
) -> Result<Option<DatabaseBranchId>> {
	let mut current_branch_id = bucket_branch_id;
	let mut walk_cap = [0xff_u8; 16];

	for _ in 0..=MAX_BUCKET_DEPTH {
		if let Some(pointer_bytes) = tx
			.informal()
			.get(
				&keys::database_pointer_cur_key(current_branch_id, database_id),
				Serializable,
			)
			.await?
		{
			let pointer = decode_database_pointer(&Vec::<u8>::from(pointer_bytes))
				.context("decode sqlite database pointer")?;
			if current_branch_id == bucket_branch_id {
				return Ok(Some(pointer.current_branch));
			}

			// A database cataloged at this level after the fork chain's cap was
			// created after the fork and is not part of the fork's view; keep
			// walking in case an older same-name database exists higher up.
			if catalog_versionstamp(tx, current_branch_id, pointer.current_branch)
				.await?
				.is_none_or(|versionstamp| versionstamp > walk_cap)
			{
				tracing::debug!(
					?current_branch_id,
					database_id,
					"skipping database cataloged after the bucket fork cap"
				);
			} else {
				// Reading the inherited source branch uncapped would show live
				// source state, so a mode that cannot write must fail instead.
				if !allow_materialize {
					return Err(SqliteStorageError::BranchNotReachable.into());
				}

				// Inherited through the parent chain: materialize a capped fork
				// owned by the accessing bucket branch.
				let materialized = materialize_inherited_database(
					tx,
					bucket_id,
					bucket_branch_id,
					database_id,
					pointer.current_branch,
					walk_cap,
					now_ms,
				)
				.await?;
				return Ok(Some(materialized));
			}
		}

		if current_branch_id == BucketBranchId::nil() {
			return Ok(None);
		}

		if tx
			.informal()
			.get(
				&keys::bucket_branches_database_name_tombstone_key(current_branch_id, database_id),
				Serializable,
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
				Serializable,
			)
			.await?
		else {
			return resolve_nil_branch_fallback(tx, database_id).await;
		};
		let record = decode_bucket_branch_record(&Vec::<u8>::from(record_bytes))
			.context("decode sqlite bucket branch record")?;
		let Some(parent) = record.parent else {
			return resolve_nil_branch_fallback(tx, database_id).await;
		};
		if let Some(parent_versionstamp) = record.parent_versionstamp {
			walk_cap = walk_cap.min(parent_versionstamp);
		}
		current_branch_id = parent;
	}

	Err(SqliteStorageError::BucketForkChainTooDeep.into())
}

/// Legacy data can live under the nil bucket branch; a failed chain walk falls
/// back to it, matching resolve_database_branch.
async fn resolve_nil_branch_fallback(
	tx: &universaldb::Transaction,
	database_id: &str,
) -> Result<Option<DatabaseBranchId>> {
	Ok(
		resolve_database_pointer(tx, BucketBranchId::nil(), database_id, Serializable)
			.await?
			.map(|pointer| pointer.current_branch),
	)
}

async fn catalog_versionstamp(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_branch_id: DatabaseBranchId,
) -> Result<Option<[u8; 16]>> {
	let Some(value) = tx
		.informal()
		.get(
			&keys::bucket_catalog_key(bucket_branch_id, database_branch_id),
			Serializable,
		)
		.await?
	else {
		return Ok(None);
	};
	let bytes = Vec::<u8>::from(value);
	let versionstamp: [u8; 16] = bytes
		.as_slice()
		.try_into()
		.context("sqlite bucket catalog versionstamp should be 16 bytes")?;

	Ok(Some(versionstamp))
}

async fn materialize_inherited_database(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	bucket_branch_id: BucketBranchId,
	database_id: &str,
	source_branch_id: DatabaseBranchId,
	walk_cap: [u8; 16],
	now_ms: i64,
) -> Result<DatabaseBranchId> {
	// A source branch with no commits at or below the cap, such as a fork that
	// never committed locally, holds no resolvable target itself; the state it
	// sees comes from its own parent. Walk up until a snap target exists so the
	// new fork derives from the branch that actually owns the history, which
	// also keeps fork chains flat.
	let mut source_branch_id = source_branch_id;
	let mut cap = walk_cap;
	let mut target = None;
	for _ in 0..=crate::conveyer::constants::MAX_FORK_DEPTH {
		let db_pins = history_pin::read_db_history_pins(tx, source_branch_id, Serializable).await?;
		if let Some(snapped) = crate::compaction::shared::snap_covered_target(
			tx,
			source_branch_id,
			cap,
			now_ms,
			&db_pins,
		)
		.await?
		{
			target = Some(snapped);
			break;
		}

		let record = super::shared::read_database_branch_record(tx, source_branch_id).await?;
		let Some(parent) = record.parent else {
			return Err(SqliteStorageError::ForkOutOfRetention.into());
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite database branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		source_branch_id = parent;
	}
	let Some((_, at_versionstamp, _)) = target else {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	};

	let new_branch_id = DatabaseBranchId::new_v4();
	derive_branch_at(
		tx,
		source_branch_id,
		at_versionstamp,
		new_branch_id,
		bucket_branch_id,
		None,
		Some((bucket_id, database_id.to_string())),
	)
	.await?;

	let pointer = DatabasePointer {
		current_branch: new_branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer =
		encode_database_pointer(pointer).context("encode sqlite materialized database pointer")?;
	tx.informal().set(
		&keys::database_pointer_cur_key(bucket_branch_id, database_id),
		&encoded_pointer,
	);
	super::catalog::write_bucket_catalog_marker(
		tx,
		bucket_branch_id,
		new_branch_id,
		&crate::conveyer::udb::INCOMPLETE_VERSIONSTAMP,
	)
	.await?;

	tracing::info!(
		?bucket_branch_id,
		?source_branch_id,
		?new_branch_id,
		"materialized inherited sqlite database for bucket fork access"
	);

	Ok(new_branch_id)
}
