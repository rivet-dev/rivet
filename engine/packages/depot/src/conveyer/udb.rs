use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	RangeOption, Subspace,
	options::{MutationType, StreamingMode},
	utils::IsolationLevel::Snapshot,
};

pub const INCOMPLETE_VERSIONSTAMP: [u8; 16] = [
	0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0,
];

pub fn compare_and_clear(tx: &universaldb::Transaction, key: &[u8], expected_value: &[u8]) {
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
	db: &universaldb::Database,
	subspace: &Subspace,
	op_counter: &AtomicUsize,
	prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	op_counter.fetch_add(1, Ordering::SeqCst);

	db.run(move |tx| {
		let subspace = subspace.clone();
		let prefix = prefix.clone();
		async move {
			let subspace_prefix = subspace.bytes().to_vec();
			let full_prefix = [subspace_prefix.as_slice(), prefix.as_slice()].concat();
			let prefix_subspace =
				Subspace::from(universaldb::tuple::Subspace::from_bytes(full_prefix));
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..RangeOption::from(&prefix_subspace)
				},
				Snapshot,
			);
			let mut rows = Vec::new();

			while let Some(entry) = stream.try_next().await? {
				let key = entry
					.key()
					.strip_prefix(subspace_prefix.as_slice())
					.context("scanned key was outside requested depot subspace")?
					.to_vec();
				rows.push((key, entry.value().to_vec()));
			}

			Ok(rows)
		}
	})
	.await
}
