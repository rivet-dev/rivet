use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{RangeOption, options::StreamingMode, utils::IsolationLevel};

use super::{
	keys,
	types::{
		DatabaseBranchId, PitrIntervalCoverage, decode_pitr_interval_coverage,
		encode_pitr_interval_coverage,
	},
};

pub fn write_pitr_interval_coverage(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
	coverage: PitrIntervalCoverage,
) -> Result<()> {
	let encoded =
		encode_pitr_interval_coverage(coverage).context("encode sqlite PITR interval coverage")?;
	tx.informal().set(
		&keys::branch_pitr_interval_key(branch_id, bucket_start_ms),
		&encoded,
	);

	Ok(())
}

pub async fn read_pitr_interval_coverage(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	bucket_start_ms: i64,
	isolation_level: IsolationLevel,
) -> Result<Option<PitrIntervalCoverage>> {
	let key = keys::branch_pitr_interval_key(branch_id, bucket_start_ms);

	tx.informal()
		.get(&key, isolation_level)
		.await?
		.map(|bytes| decode_pitr_interval_coverage(&bytes))
		.transpose()
		.context("decode sqlite PITR interval coverage")
}

pub async fn scan_pitr_interval_coverage(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: IsolationLevel,
) -> Result<Vec<(i64, PitrIntervalCoverage)>> {
	let rows = read_prefix_values(
		tx,
		&keys::branch_pitr_interval_prefix(branch_id),
		isolation_level,
	)
	.await?;

	rows.into_iter()
		.map(|(key, value)| {
			let bucket_start_ms = keys::decode_branch_pitr_interval_bucket(branch_id, &key)?;
			let coverage = decode_pitr_interval_coverage(&value)
				.context("decode sqlite PITR interval coverage")?;
			Ok((bucket_start_ms, coverage))
		})
		.collect()
}

pub async fn read_latest_pitr_interval_coverage_at_or_before(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	timestamp_ms: i64,
	isolation_level: IsolationLevel,
) -> Result<Option<(i64, PitrIntervalCoverage)>> {
	let rows = scan_pitr_interval_coverage(tx, branch_id, isolation_level).await?;

	Ok(rows.into_iter().rev().find(|(bucket_start_ms, coverage)| {
		*bucket_start_ms <= timestamp_ms && coverage.wall_clock_ms <= timestamp_ms
	}))
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
