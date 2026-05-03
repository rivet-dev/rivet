use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{RangeOption, options::StreamingMode, utils::IsolationLevel};

use super::{
	keys,
	types::{
		BucketBranchId, DatabaseBranchId, DbHistoryPin, DbHistoryPinKind, RestorePointId,
		decode_db_history_pin, encode_db_history_pin,
	},
};

pub fn database_fork_pin_id(owner_database_branch_id: DatabaseBranchId) -> Vec<u8> {
	let mut pin_id = b"database_fork/".to_vec();
	pin_id.extend_from_slice(owner_database_branch_id.as_uuid().as_bytes());
	pin_id
}

pub fn restore_point_pin_id(restore_point: &RestorePointId) -> Vec<u8> {
	let mut pin_id = b"restore_point/".to_vec();
	pin_id.extend_from_slice(restore_point.as_str().as_bytes());
	pin_id
}

pub fn bucket_fork_pin_id(owner_bucket_branch_id: BucketBranchId) -> Vec<u8> {
	let mut pin_id = b"bucket_fork/".to_vec();
	pin_id.extend_from_slice(owner_bucket_branch_id.as_uuid().as_bytes());
	pin_id
}

pub fn write_database_fork_pin(
	tx: &universaldb::Transaction,
	source_branch_id: DatabaseBranchId,
	owner_database_branch_id: DatabaseBranchId,
	at_versionstamp: [u8; 16],
	at_txid: u64,
	created_at_ms: i64,
) -> Result<()> {
	write_db_history_pin(
		tx,
		source_branch_id,
		&database_fork_pin_id(owner_database_branch_id),
		DbHistoryPin {
			at_versionstamp,
			at_txid,
			kind: DbHistoryPinKind::DatabaseFork,
			owner_database_branch_id: Some(owner_database_branch_id),
			owner_bucket_branch_id: None,
			owner_restore_point: None,
			created_at_ms,
		},
	)
}

pub fn write_restore_point_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	restore_point: RestorePointId,
	at_versionstamp: [u8; 16],
	at_txid: u64,
	created_at_ms: i64,
) -> Result<()> {
	write_db_history_pin(
		tx,
		branch_id,
		&restore_point_pin_id(&restore_point),
		DbHistoryPin {
			at_versionstamp,
			at_txid,
			kind: DbHistoryPinKind::RestorePoint,
			owner_database_branch_id: None,
			owner_bucket_branch_id: None,
			owner_restore_point: Some(restore_point),
			created_at_ms,
		},
	)
}

pub fn write_bucket_fork_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	owner_bucket_branch_id: BucketBranchId,
	at_versionstamp: [u8; 16],
	at_txid: u64,
	created_at_ms: i64,
) -> Result<()> {
	write_db_history_pin(
		tx,
		branch_id,
		&bucket_fork_pin_id(owner_bucket_branch_id),
		DbHistoryPin {
			at_versionstamp,
			at_txid,
			kind: DbHistoryPinKind::BucketFork,
			owner_database_branch_id: None,
			owner_bucket_branch_id: Some(owner_bucket_branch_id),
			owner_restore_point: None,
			created_at_ms,
		},
	)
}

pub fn write_db_history_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	pin_id: &[u8],
	pin: DbHistoryPin,
) -> Result<()> {
	let encoded = encode_db_history_pin(pin).context("encode sqlite db history pin")?;
	tx.informal()
		.set(&keys::db_pin_key(branch_id, pin_id), &encoded);

	Ok(())
}

pub fn delete_restore_point_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	restore_point: &RestorePointId,
) {
	delete_db_history_pin(tx, branch_id, &restore_point_pin_id(restore_point));
}

pub fn delete_db_history_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	pin_id: &[u8],
) {
	tx.informal().clear(&keys::db_pin_key(branch_id, pin_id));
}

pub async fn read_db_history_pins(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: IsolationLevel,
) -> Result<Vec<DbHistoryPin>> {
	let rows = read_prefix_values(tx, &keys::db_pin_prefix(branch_id), isolation_level).await?;

	rows.into_iter()
		.map(|(_, value)| decode_db_history_pin(&value))
		.collect::<Result<Vec<_>>>()
		.context("decode sqlite db history pins")
}

async fn read_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
	isolation_level: IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}
