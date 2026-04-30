use std::sync::atomic::AtomicUsize;

use anyhow::{Context, Result, bail};
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

pub fn append_versionstamp_offset(mut bytes: Vec<u8>, versionstamp: &[u8; 16]) -> Result<Vec<u8>> {
	let offset = bytes
		.windows(versionstamp.len())
		.position(|window| window == versionstamp)
		.context("versionstamp placeholder not found")?;
	let offset = u32::try_from(offset).context("versionstamp offset exceeded u32")?;
	bytes.extend_from_slice(&offset.to_le_bytes());
	Ok(bytes)
}

pub async fn scan_prefix_values(
	_db: &universaldb::Database,
	_subspace: &Subspace,
	_op_counter: &AtomicUsize,
	_prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	bail!("sqlite-storage pump UDB scan helpers are not implemented yet")
}
