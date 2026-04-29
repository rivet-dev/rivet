use std::sync::atomic::AtomicUsize;

use anyhow::{Result, bail};
use universaldb::options::MutationType;
use universaldb::Subspace;

pub fn compare_and_clear(
	tx: &universaldb::Transaction,
	key: &[u8],
	expected_value: &[u8],
) {
	tx.informal()
		.atomic_op(key, expected_value, MutationType::CompareAndClear);
}

pub async fn scan_prefix_values(
	_db: &universaldb::Database,
	_subspace: &Subspace,
	_op_counter: &AtomicUsize,
	_prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	bail!("sqlite-storage pump UDB scan helpers are not implemented yet")
}
