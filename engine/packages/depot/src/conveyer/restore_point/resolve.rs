use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Snapshot;

use crate::conveyer::{
	branch,
	error::SqliteStorageError,
	keys, pitr_interval,
	types::{
		BucketBranchId, BucketId, DatabaseBranchId, PinStatus, ResolvedRestoreTarget,
		ResolvedVersionstamp, RestorePointId, RestorePointRef, SnapshotKind, SnapshotSelector,
		decode_bucket_branch_record, decode_commit_row, decode_database_pointer, decode_db_head,
		decode_restore_point_record,
	},
};

const VERSIONSTAMP_INFINITY: [u8; 16] = [0xff; 16];

pub async fn resolve_restore_point(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	restore_point: RestorePointId,
) -> Result<ResolvedVersionstamp> {
	udb.run(move |tx| {
		let database_id = database_id.clone();
		let restore_point = restore_point.clone();

		async move {
			let (branch_id, bucket_cap) =
				resolve_visible_database_branch_for_restore_point(&tx, bucket_id, &database_id)
					.await?;
			resolve_restore_point_in_branch_chain(
				&tx,
				&database_id,
				branch_id,
				bucket_cap,
				restore_point,
			)
			.await
		}
	})
	.await
}

pub async fn resolve_restore_target(
	udb: &universaldb::Database,
	bucket_id: BucketId,
	database_id: String,
	selector: SnapshotSelector,
) -> Result<ResolvedRestoreTarget> {
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let database_id = database_id.clone();
		let selector = selector.clone();

		async move {
			let (branch_id, bucket_cap) =
				resolve_visible_database_branch_for_restore_point(&tx, bucket_id, &database_id)
					.await?;
			match selector {
				SnapshotSelector::Latest => {
					resolve_latest_in_branch_chain(&tx, branch_id, bucket_cap).await
				}
				SnapshotSelector::AtTimestamp { timestamp_ms } => {
					resolve_timestamp_in_branch_chain(
						&tx,
						branch_id,
						bucket_cap,
						timestamp_ms,
						now_ms,
					)
					.await
				}
				SnapshotSelector::RestorePoint { restore_point } => {
					resolve_restore_point_target_in_branch_chain(
						&tx,
						&database_id,
						branch_id,
						bucket_cap,
						restore_point,
					)
					.await
				}
			}
		}
	})
	.await
}

async fn resolve_visible_database_branch_for_restore_point(
	tx: &universaldb::Transaction,
	bucket_id: BucketId,
	database_id: &str,
) -> Result<(DatabaseBranchId, [u8; 16])> {
	let Some(mut bucket_branch_id) = branch::resolve_bucket_branch(tx, bucket_id, Snapshot).await?
	else {
		if let Some(pointer) =
			branch::resolve_database_pointer(tx, BucketBranchId::nil(), database_id, Snapshot)
				.await?
		{
			return Ok((pointer.current_branch, VERSIONSTAMP_INFINITY));
		}

		return Err(SqliteStorageError::BranchNotReachable.into());
	};

	let mut cap = VERSIONSTAMP_INFINITY;
	for _ in 0..=crate::constants::MAX_BUCKET_DEPTH {
		if let Some(pointer_bytes) = tx
			.informal()
			.get(
				&keys::database_pointer_cur_key(bucket_branch_id, database_id),
				Snapshot,
			)
			.await?
		{
			let pointer = decode_database_pointer(&pointer_bytes)
				.context("decode sqlite database pointer during restore point resolution")?;
			return Ok((pointer.current_branch, cap));
		}

		if tx
			.informal()
			.get(
				&keys::bucket_branches_database_name_tombstone_key(bucket_branch_id, database_id),
				Snapshot,
			)
			.await?
			.is_some()
		{
			return Err(SqliteStorageError::BranchNotReachable.into());
		}

		let Some(record_bytes) = tx
			.informal()
			.get(&keys::bucket_branches_list_key(bucket_branch_id), Snapshot)
			.await?
		else {
			return Err(SqliteStorageError::BranchNotReachable.into());
		};
		let record = decode_bucket_branch_record(&record_bytes)
			.context("decode sqlite bucket branch record during restore point resolution")?;
		let Some(parent) = record.parent else {
			return Err(SqliteStorageError::BranchNotReachable.into());
		};
		if let Some(parent_versionstamp) = record.parent_versionstamp {
			cap = cap.min(parent_versionstamp);
		}
		bucket_branch_id = parent;
	}

	Err(SqliteStorageError::BucketForkChainTooDeep.into())
}

async fn resolve_latest_in_branch_chain(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	bucket_cap: [u8; 16],
) -> Result<ResolvedRestoreTarget> {
	let mut current_branch_id = branch_id;
	let mut cap = bucket_cap;
	for _ in 0..=crate::constants::MAX_FORK_DEPTH {
		if let Some(target) = read_latest_target_in_branch(tx, current_branch_id, cap).await? {
			return Ok(target);
		}

		let record = branch::read_database_branch_record(tx, current_branch_id).await?;
		let Some(parent) = record.parent else {
			break;
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite database branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		current_branch_id = parent;
	}

	Err(SqliteStorageError::RestoreTargetExpired.into())
}

async fn read_latest_target_in_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cap: [u8; 16],
) -> Result<Option<ResolvedRestoreTarget>> {
	if let Some(head_bytes) = tx
		.informal()
		.get(&keys::branch_meta_head_key(branch_id), Snapshot)
		.await?
	{
		let head = decode_db_head(&head_bytes).context("decode sqlite database branch head")?;
		let commit = read_commit_row(tx, branch_id, head.head_txid).await?;
		if commit.versionstamp <= cap {
			return Ok(Some(ResolvedRestoreTarget {
				database_branch_id: branch_id,
				txid: head.head_txid,
				versionstamp: commit.versionstamp,
				wall_clock_ms: commit.wall_clock_ms,
				kind: SnapshotKind::Latest,
				restore_point: None,
			}));
		}

		let txid = lookup_txid_for_versionstamp(tx, branch_id, cap).await?;
		let capped_commit = read_commit_row(tx, branch_id, txid).await?;
		return Ok(Some(ResolvedRestoreTarget {
			database_branch_id: branch_id,
			txid,
			versionstamp: capped_commit.versionstamp,
			wall_clock_ms: capped_commit.wall_clock_ms,
			kind: SnapshotKind::Latest,
			restore_point: None,
		}));
	}

	Ok(None)
}

async fn resolve_timestamp_in_branch_chain(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	bucket_cap: [u8; 16],
	timestamp_ms: i64,
	now_ms: i64,
) -> Result<ResolvedRestoreTarget> {
	let mut current_branch_id = branch_id;
	let mut cap = bucket_cap;
	let mut best: Option<ResolvedRestoreTarget> = None;
	for _ in 0..=crate::constants::MAX_FORK_DEPTH {
		if let Some(target) =
			read_timestamp_target_in_branch(tx, current_branch_id, cap, timestamp_ms, now_ms)
				.await?
		{
			let should_replace = match best.as_ref() {
				Some(best) => target.wall_clock_ms > best.wall_clock_ms,
				None => true,
			};
			if should_replace {
				best = Some(target);
			}
		}

		let record = branch::read_database_branch_record(tx, current_branch_id).await?;
		let Some(parent) = record.parent else {
			break;
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite database branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		current_branch_id = parent;
	}

	best.ok_or_else(|| SqliteStorageError::RestoreTargetExpired.into())
}

async fn read_timestamp_target_in_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cap: [u8; 16],
	timestamp_ms: i64,
	now_ms: i64,
) -> Result<Option<ResolvedRestoreTarget>> {
	let rows = pitr_interval::scan_pitr_interval_coverage(tx, branch_id, Snapshot).await?;
	let Some((_bucket_start_ms, coverage)) =
		rows.into_iter().rev().find(|(bucket_start_ms, coverage)| {
			*bucket_start_ms <= timestamp_ms
				&& coverage.wall_clock_ms <= timestamp_ms
				&& coverage.expires_at_ms > now_ms
				&& coverage.versionstamp <= cap
		})
	else {
		return Ok(None);
	};

	Ok(Some(ResolvedRestoreTarget {
		database_branch_id: branch_id,
		txid: coverage.txid,
		versionstamp: coverage.versionstamp,
		wall_clock_ms: coverage.wall_clock_ms,
		kind: SnapshotKind::AtTimestamp,
		restore_point: None,
	}))
}

async fn resolve_restore_point_in_branch_chain(
	tx: &universaldb::Transaction,
	database_id: &str,
	branch_id: DatabaseBranchId,
	bucket_cap: [u8; 16],
	restore_point: RestorePointId,
) -> Result<ResolvedVersionstamp> {
	let pinned_record = tx
		.informal()
		.get(
			&keys::restore_point_key(database_id, restore_point.as_str()),
			Snapshot,
		)
		.await?
		.map(|bytes| {
			decode_restore_point_record(&bytes)
				.context("decode sqlite restore point record during restore point resolution")
		})
		.transpose()?
		.ok_or(SqliteStorageError::RestorePointNotFound)?;

	let mut current_branch_id = branch_id;
	let mut cap = bucket_cap;
	for _ in 0..=crate::constants::MAX_FORK_DEPTH {
		if pinned_record.database_branch_id == current_branch_id
			&& pinned_record.versionstamp <= cap
		{
			return Ok(ResolvedVersionstamp {
				versionstamp: pinned_record.versionstamp,
				restore_point: Some(RestorePointRef {
					restore_point: restore_point.clone(),
					resolved_versionstamp: Some(pinned_record.versionstamp),
				}),
			});
		}

		let record = branch::read_database_branch_record(tx, current_branch_id).await?;
		let Some(parent) = record.parent else {
			break;
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite database branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		current_branch_id = parent;
	}

	Err(SqliteStorageError::BranchNotReachable.into())
}

async fn resolve_restore_point_target_in_branch_chain(
	tx: &universaldb::Transaction,
	database_id: &str,
	branch_id: DatabaseBranchId,
	bucket_cap: [u8; 16],
	restore_point: RestorePointId,
) -> Result<ResolvedRestoreTarget> {
	let pinned_record = tx
		.informal()
		.get(
			&keys::restore_point_key(database_id, restore_point.as_str()),
			Snapshot,
		)
		.await?
		.map(|bytes| {
			decode_restore_point_record(&bytes)
				.context("decode sqlite restore point record during restore target resolution")
		})
		.transpose()?
		.ok_or(SqliteStorageError::RestorePointNotFound)?;
	if pinned_record.status != PinStatus::Ready {
		return Err(SqliteStorageError::RestorePointNotFound.into());
	}

	let (_, txid) = restore_point.parse()?;
	let mut current_branch_id = branch_id;
	let mut cap = bucket_cap;
	for _ in 0..=crate::constants::MAX_FORK_DEPTH {
		if pinned_record.database_branch_id == current_branch_id
			&& pinned_record.versionstamp <= cap
		{
			let commit = read_commit_row(tx, current_branch_id, txid).await?;
			return Ok(ResolvedRestoreTarget {
				database_branch_id: current_branch_id,
				txid,
				versionstamp: pinned_record.versionstamp,
				wall_clock_ms: commit.wall_clock_ms,
				kind: SnapshotKind::RestorePoint,
				restore_point: Some(RestorePointRef {
					restore_point: restore_point.clone(),
					resolved_versionstamp: Some(pinned_record.versionstamp),
				}),
			});
		}

		let record = branch::read_database_branch_record(tx, current_branch_id).await?;
		let Some(parent) = record.parent else {
			break;
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite database branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		current_branch_id = parent;
	}

	Err(SqliteStorageError::BranchNotReachable.into())
}

async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<crate::conveyer::types::CommitRow> {
	let bytes = tx
		.informal()
		.get(&keys::branch_commit_key(branch_id, txid), Snapshot)
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;

	decode_commit_row(&bytes).context("decode sqlite commit row")
}

async fn lookup_txid_for_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = tx
		.informal()
		.get(&keys::branch_vtx_key(branch_id, versionstamp), Snapshot)
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?
		.as_millis();
	i64::try_from(millis).context("current timestamp exceeded i64 milliseconds")
}
