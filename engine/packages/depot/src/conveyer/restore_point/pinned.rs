use anyhow::{Context, Result};
use universaldb::options::MutationType;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};

use crate::conveyer::{
	branch,
	constants::MAX_RESTORE_POINTS_PER_BUCKET,
	error::SqliteStorageError,
	history_pin, keys,
	restore_point::{
		recompute::recompute_database_branch_restore_point_pin,
		resolve,
		shared::{ResolvedRestorePointPin, RestorePointCreateResult, decode_i64_counter},
		test_hooks,
	},
	types::{
		BucketBranchId, BucketId, PinStatus, RestorePointId, RestorePointRecord, SnapshotSelector,
		decode_restore_point_record, encode_restore_point_record,
	},
};

pub async fn create_restore_point(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	selector: SnapshotSelector,
) -> Result<RestorePointId> {
	let target =
		resolve::resolve_restore_target(udb, bucket_id, database_id.clone(), selector).await?;
	test_hooks::maybe_pause_after_resolve(&database_id).await;
	let restore_point = RestorePointId::format(target.wall_clock_ms, target.txid)?;
	let result = create_restore_point_for_resolved(
		udb,
		bucket_id,
		database_id,
		ResolvedRestorePointPin {
			restore_point,
			database_branch_id: target.database_branch_id,
			versionstamp: target.versionstamp,
			created_at_ms: target.wall_clock_ms,
		},
	)
	.await?;

	Ok(result.restore_point)
}

pub async fn delete_restore_point(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	restore_point: RestorePointId,
) -> Result<()> {
	udb.run(move |tx| {
		let database_id = database_id.clone();
		let restore_point = restore_point.clone();

		async move {
			let pinned_key = keys::restore_point_key(&database_id, restore_point.as_str());
			let Some(pinned_bytes) = tx.informal().get(&pinned_key, Serializable).await? else {
				return Ok(());
			};
			let pinned = decode_restore_point_record(&pinned_bytes)
				.context("decode sqlite restore point record")?;
			let bucket_branch_id = branch::resolve_bucket_branch(&tx, bucket_id, Serializable)
				.await?
				.unwrap_or_else(BucketBranchId::nil);
			let pin_count_key = keys::bucket_branches_pin_count_key(bucket_branch_id);

			tx.informal().clear(&pinned_key);
			history_pin::delete_restore_point_pin(&tx, pinned.database_branch_id, &restore_point);
			tx.informal()
				.atomic_op(&pin_count_key, &(-1_i64).to_le_bytes(), MutationType::Add);

			let recomputed_pin = recompute_database_branch_restore_point_pin(
				&tx,
				&database_id,
				pinned.database_branch_id,
				&pinned_key,
			)
			.await?;
			let branch_pin_key = keys::branches_restore_point_pin_key(pinned.database_branch_id);
			if let Some(recomputed_pin) = recomputed_pin {
				tx.informal().set(&branch_pin_key, &recomputed_pin);
			} else {
				tx.informal().clear(&branch_pin_key);
			}

			Ok(())
		}
	})
	.await?;

	Ok(())
}

pub async fn restore_point_status(
	udb: &universaldb::Database,
	_bucket_id: BucketId,
	database_id: String,
	restore_point: RestorePointId,
) -> Result<Option<PinStatus>> {
	udb.run(move |tx| {
		let database_id = database_id.clone();
		let restore_point = restore_point.clone();

		async move {
			let Some(bytes) = tx
				.informal()
				.get(
					&keys::restore_point_key(&database_id, restore_point.as_str()),
					Snapshot,
				)
				.await?
			else {
				return Ok(None);
			};
			let record = decode_restore_point_record(&bytes)
				.context("decode sqlite restore point record")?;

			Ok(Some(record.status))
		}
	})
	.await
}

pub(super) async fn create_restore_point_for_resolved(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	pin: ResolvedRestorePointPin,
) -> Result<RestorePointCreateResult> {
	udb.run(move |tx| {
		let database_id = database_id.clone();
		let pin = pin.clone();

		async move { create_restore_point_for_resolved_tx(&tx, bucket_id, &database_id, &pin).await }
	})
	.await
}

pub(super) async fn create_restore_point_for_resolved_tx(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
	pin: &ResolvedRestorePointPin,
) -> Result<RestorePointCreateResult> {
	let bucket_branch_id = branch::resolve_bucket_branch(tx, bucket_id, Serializable)
		.await?
		.unwrap_or_else(BucketBranchId::nil);
	let pinned_key = keys::restore_point_key(database_id, pin.restore_point.as_str());
	let (_, restore_point_txid) = pin.restore_point.parse()?;
	// The commit row is the history that makes this Ready pin resolvable. Re-read it
	// in this write transaction so reclaim cannot delete it between target resolution
	// and pin creation.
	tx.informal()
		.get(
			&keys::branch_commit_key(pin.database_branch_id, restore_point_txid),
			Serializable,
		)
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;

	if tx
		.informal()
		.get(&pinned_key, Serializable)
		.await?
		.is_none()
	{
		let pin_count_key = keys::bucket_branches_pin_count_key(bucket_branch_id);
		let pin_count = tx
			.informal()
			.get(&pin_count_key, Serializable)
			.await?
			.map(|bytes| decode_i64_counter(&bytes))
			.transpose()?
			.unwrap_or(0);
		if pin_count >= i64::from(MAX_RESTORE_POINTS_PER_BUCKET) {
			return Err(SqliteStorageError::TooManyRestorePoints.into());
		}

		let record = RestorePointRecord {
			restore_point_id: pin.restore_point.clone(),
			database_branch_id: pin.database_branch_id,
			versionstamp: pin.versionstamp,
			status: PinStatus::Ready,
			pin_object_key: None,
			created_at_ms: pin.created_at_ms,
			updated_at_ms: pin.created_at_ms,
		};
		let encoded =
			encode_restore_point_record(record).context("encode sqlite restore point record")?;
		tx.informal().set(&pinned_key, &encoded);
		history_pin::write_restore_point_pin(
			tx,
			pin.database_branch_id,
			pin.restore_point.clone(),
			pin.versionstamp,
			restore_point_txid,
			pin.created_at_ms,
		)?;
		tx.informal()
			.atomic_op(&pin_count_key, &1_i64.to_le_bytes(), MutationType::Add);
	}
	tx.informal().atomic_op(
		&keys::branches_restore_point_pin_key(pin.database_branch_id),
		&pin.versionstamp,
		MutationType::ByteMin,
	);

	Ok(RestorePointCreateResult {
		restore_point: pin.restore_point.clone(),
	})
}
