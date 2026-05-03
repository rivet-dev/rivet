use std::collections::BTreeSet;

use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel::Serializable};

use super::{
	resolve::resolve_bucket_branch,
	shared::{decode_versionstamp_value, read_bucket_branch_record, tx_scan_prefix_values},
};
use crate::conveyer::{
	constants::MAX_BUCKET_DEPTH,
	error::SqliteStorageError,
	keys,
	types::{
		BucketBranchId, BucketCatalogDbFact, BucketForkFact, BucketId, DatabaseBranchId,
		decode_bucket_catalog_db_fact, encode_bucket_catalog_db_fact, encode_bucket_fork_fact,
	},
	udb,
};

pub async fn list_databases(
	udb: &universaldb::Database,
	bucket: BucketId,
) -> Result<Vec<DatabaseBranchId>> {
	udb.run(move |tx| async move {
		let Some(bucket_branch_id) = resolve_bucket_branch(&tx, bucket, Serializable).await? else {
			return Ok(Vec::new());
		};

		list_databases_in_bucket_branch(&tx, bucket_branch_id).await
	})
	.await
}

async fn list_databases_in_bucket_branch(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
) -> Result<Vec<DatabaseBranchId>> {
	let mut result = BTreeSet::new();
	let mut tombstones = BTreeSet::new();
	let mut current_branch_id = bucket_branch_id;
	let mut versionstamp_cap = [0xff; 16];

	for depth in 0..=MAX_BUCKET_DEPTH {
		for (database_id, tombstone_versionstamp) in
			scan_database_tombstones(tx, current_branch_id).await?
		{
			if tombstone_versionstamp <= versionstamp_cap {
				tombstones.insert(database_id);
				result.remove(&database_id);
			}
		}

		for (database_id, catalog_versionstamp) in
			scan_bucket_catalog(tx, current_branch_id).await?
		{
			if catalog_versionstamp <= versionstamp_cap && !tombstones.contains(&database_id) {
				result.insert(database_id);
			}
		}

		let record = read_bucket_branch_record(tx, current_branch_id).await?;
		let Some(parent_branch_id) = record.parent else {
			return Ok(result.into_iter().collect());
		};
		if depth == MAX_BUCKET_DEPTH {
			return Err(SqliteStorageError::BucketForkChainTooDeep.into());
		}

		versionstamp_cap = record
			.parent_versionstamp
			.context("sqlite bucket branch parent versionstamp is missing")?;
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::BucketForkChainTooDeep.into())
}

pub(super) async fn is_database_visible_in_bucket_branch(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
) -> Result<bool> {
	Ok(list_databases_in_bucket_branch(tx, bucket_branch_id)
		.await?
		.contains(&database_id))
}

pub(crate) async fn write_bucket_catalog_marker(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
	catalog_versionstamp: &[u8; 16],
) -> Result<()> {
	let root_bucket_branch_id = read_bucket_root_branch_id(tx, bucket_branch_id).await?;
	write_bucket_catalog_marker_with_root(
		tx,
		root_bucket_branch_id,
		bucket_branch_id,
		database_id,
		catalog_versionstamp,
	)
}

pub(crate) fn write_bucket_catalog_marker_with_root(
	tx: &universaldb::Transaction,
	root_bucket_branch_id: BucketBranchId,
	bucket_branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
	catalog_versionstamp: &[u8; 16],
) -> Result<()> {
	tx.informal().atomic_op(
		&keys::bucket_catalog_key(bucket_branch_id, database_id),
		&udb::append_versionstamp_offset(catalog_versionstamp.to_vec(), catalog_versionstamp)
			.context("prepare versionstamped sqlite bucket catalog marker")?,
		MutationType::SetVersionstampedValue,
	);
	let fact = BucketCatalogDbFact {
		database_branch_id: database_id,
		bucket_branch_id,
		catalog_versionstamp: *catalog_versionstamp,
		tombstone_versionstamp: None,
	};
	let encoded_fact =
		encode_bucket_catalog_db_fact(fact).context("encode sqlite bucket catalog proof fact")?;
	tx.informal().atomic_op(
		&keys::bucket_catalog_by_db_key(database_id, bucket_branch_id),
		&udb::append_versionstamp_offset(encoded_fact, catalog_versionstamp)
			.context("prepare versionstamped sqlite bucket catalog proof fact")?,
		MutationType::SetVersionstampedValue,
	);
	bump_bucket_proof_epoch(tx, root_bucket_branch_id);

	Ok(())
}

pub(super) async fn write_bucket_catalog_tombstone_marker(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
	database_id: DatabaseBranchId,
) -> Result<()> {
	let root_bucket_branch_id = read_bucket_root_branch_id(tx, bucket_branch_id).await?;
	let existing_fact = tx
		.informal()
		.get(
			&keys::bucket_catalog_by_db_key(database_id, bucket_branch_id),
			Serializable,
		)
		.await?
		.as_deref()
		.map(|bytes| decode_bucket_catalog_db_fact(bytes))
		.transpose()
		.context("decode sqlite bucket catalog proof fact before tombstone")?;
	let fact = BucketCatalogDbFact {
		database_branch_id: database_id,
		bucket_branch_id,
		catalog_versionstamp: existing_fact
			.as_ref()
			.map_or([0; 16], |fact| fact.catalog_versionstamp),
		tombstone_versionstamp: Some(udb::INCOMPLETE_VERSIONSTAMP),
	};
	let encoded_fact = encode_bucket_catalog_db_fact(fact)
		.context("encode sqlite bucket catalog tombstone proof fact")?;
	tx.informal().atomic_op(
		&keys::bucket_catalog_by_db_key(database_id, bucket_branch_id),
		&udb::append_versionstamp_offset(encoded_fact, &udb::INCOMPLETE_VERSIONSTAMP)
			.context("prepare versionstamped sqlite bucket catalog tombstone proof fact")?,
		MutationType::SetVersionstampedValue,
	);
	bump_bucket_proof_epoch(tx, root_bucket_branch_id);

	Ok(())
}

pub(super) async fn write_bucket_fork_facts(
	tx: &universaldb::Transaction,
	source_bucket_branch_id: BucketBranchId,
	target_bucket_branch_id: BucketBranchId,
	fork_versionstamp: [u8; 16],
) -> Result<()> {
	let root_bucket_branch_id = read_bucket_root_branch_id(tx, source_bucket_branch_id).await?;
	let fact = BucketForkFact {
		source_bucket_branch_id,
		target_bucket_branch_id,
		fork_versionstamp,
		parent_cap_versionstamp: fork_versionstamp,
	};
	let encoded_fact =
		encode_bucket_fork_fact(fact).context("encode sqlite bucket fork proof fact")?;
	tx.informal().set(
		&keys::bucket_fork_pin_key(
			source_bucket_branch_id,
			fork_versionstamp,
			target_bucket_branch_id,
		),
		&encoded_fact,
	);
	tx.informal().set(
		&keys::bucket_child_key(
			source_bucket_branch_id,
			fork_versionstamp,
			target_bucket_branch_id,
		),
		&encoded_fact,
	);
	bump_bucket_proof_epoch(tx, root_bucket_branch_id);

	Ok(())
}

async fn read_bucket_root_branch_id(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
) -> Result<BucketBranchId> {
	let mut current_branch_id = bucket_branch_id;

	for depth in 0..=MAX_BUCKET_DEPTH {
		let record = read_bucket_branch_record(tx, current_branch_id).await?;
		let Some(parent_branch_id) = record.parent else {
			return Ok(current_branch_id);
		};
		if depth == MAX_BUCKET_DEPTH {
			return Err(SqliteStorageError::BucketForkChainTooDeep.into());
		}
		current_branch_id = parent_branch_id;
	}

	Err(SqliteStorageError::BucketForkChainTooDeep.into())
}

fn bump_bucket_proof_epoch(tx: &universaldb::Transaction, root_bucket_branch_id: BucketBranchId) {
	tx.informal().atomic_op(
		&keys::bucket_proof_epoch_key(root_bucket_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
}

pub(super) fn versionstamped_marker_value() -> Result<Vec<u8>> {
	udb::append_versionstamp_offset(
		udb::INCOMPLETE_VERSIONSTAMP.to_vec(),
		&udb::INCOMPLETE_VERSIONSTAMP,
	)
}

async fn scan_bucket_catalog(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
) -> Result<Vec<(DatabaseBranchId, [u8; 16])>> {
	let rows = tx_scan_prefix_values(tx, &keys::bucket_catalog_prefix(bucket_branch_id)).await?;
	rows.into_iter()
		.map(|(key, value)| {
			let database_id = keys::decode_bucket_catalog_database_id(bucket_branch_id, &key)?;
			let versionstamp = decode_versionstamp_value(&value)
				.context("decode sqlite bucket catalog versionstamp")?;

			Ok((database_id, versionstamp))
		})
		.collect()
}

async fn scan_database_tombstones(
	tx: &universaldb::Transaction,
	bucket_branch_id: BucketBranchId,
) -> Result<Vec<(DatabaseBranchId, [u8; 16])>> {
	let rows = tx_scan_prefix_values(
		tx,
		&keys::bucket_branches_database_tombstone_prefix(bucket_branch_id),
	)
	.await?;
	rows.into_iter()
		.map(|(key, value)| {
			let database_id =
				keys::decode_bucket_branches_database_tombstone_id(bucket_branch_id, &key)?;
			let versionstamp = if value.is_empty() {
				[0; 16]
			} else {
				decode_versionstamp_value(&value)
					.context("decode sqlite bucket database tombstone versionstamp")?
			};

			Ok((database_id, versionstamp))
		})
		.collect()
}
