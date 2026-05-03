use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{RangeOption, options::StreamingMode, utils::IsolationLevel::Serializable};

use crate::conveyer::{
	error::SqliteStorageError,
	keys,
	types::{
		BucketBranchId, BucketBranchRecord, CommitRow, DatabaseBranchId, DatabaseBranchRecord,
		decode_bucket_branch_record, decode_commit_row, decode_database_branch_record,
	},
};

pub(super) fn decode_versionstamp_value(bytes: &[u8]) -> Result<[u8; 16]> {
	bytes
		.try_into()
		.context("sqlite versionstamp value should be exactly 16 bytes")
}

pub(super) async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Serializable,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

pub(in crate::conveyer) async fn read_database_branch_record(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<DatabaseBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite database branch record is missing")?;

	decode_database_branch_record(&bytes).context("decode sqlite database branch record")
}

pub(super) async fn read_bucket_branch_record(
	tx: &universaldb::Transaction,
	branch_id: BucketBranchId,
) -> Result<BucketBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::bucket_branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite bucket branch record is missing")?;

	decode_bucket_branch_record(&bytes).context("decode sqlite bucket branch record")
}

pub(super) async fn read_versionstamp_pin(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<[u8; 16]> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok([0; 16]);
	};
	let bytes = Vec::<u8>::from(bytes);
	bytes
		.as_slice()
		.try_into()
		.context("sqlite branch pin should be exactly 16 bytes")
}

pub(super) async fn lookup_txid_at_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = tx
		.informal()
		.get(&keys::branch_vtx_key(branch_id, versionstamp), Serializable)
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;
	let bytes = Vec::<u8>::from(bytes);
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
}

pub(super) async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
) -> Result<CommitRow> {
	let bytes = tx
		.informal()
		.get(&keys::branch_commit_key(branch_id, txid), Serializable)
		.await?
		.ok_or(SqliteStorageError::RestoreTargetExpired)?;

	decode_commit_row(&bytes).context("decode sqlite commit row")
}

pub(super) fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?
		.as_millis();
	i64::try_from(millis).context("current timestamp exceeded i64 milliseconds")
}
