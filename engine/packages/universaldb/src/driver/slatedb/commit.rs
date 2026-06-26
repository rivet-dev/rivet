use std::{
	collections::BTreeMap,
	ops::Bound,
	sync::Arc,
};

use anyhow::{Context, Result};
use slatedb::{
	Db, WriteBatch,
	config::{DurabilityLevel, ReadOptions, ScanOptions},
};

use crate::{
	atomic::apply_atomic_op,
	options::MutationType,
	tx_ops::Operation,
	versionstamp::{generate_versionstamp, substitute_raw_versionstamp},
};

fn memory_read_options() -> ReadOptions {
	ReadOptions::new()
		.with_durability_filter(DurabilityLevel::Memory)
		.with_dirty(false)
}

fn memory_scan_options() -> ScanOptions {
	ScanOptions::new()
		.with_durability_filter(DurabilityLevel::Memory)
		.with_dirty(false)
}

fn range_contains(begin: &[u8], end: &[u8], key: &[u8]) -> bool {
	begin <= key && key < end
}

fn overlay_keys_in_range(
	overlay: &BTreeMap<Vec<u8>, Option<Vec<u8>>>,
	begin: &[u8],
	end: &[u8],
) -> Vec<Vec<u8>> {
	overlay
		.range::<[u8], _>((Bound::Included(begin), Bound::Excluded(end)))
		.map(|(key, _)| key.clone())
		.collect()
}

fn cleared_ranges_cover(cleared: &[(Vec<u8>, Vec<u8>)], key: &[u8]) -> bool {
	cleared
		.iter()
		.any(|(begin, end)| range_contains(begin, end, key))
}

async fn get_live(db: &Db, key: &[u8]) -> Result<Option<Vec<u8>>> {
	Ok(db
		.get_with_options(key, &memory_read_options())
		.await
		.context("failed to read SlateDB value")?
		.map(|value| value.to_vec()))
}

pub async fn build_write_batch(db: Arc<Db>, operations: Vec<Operation>) -> Result<WriteBatch> {
	let versionstamp = generate_versionstamp(0);
	let mut overlay: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();
	let mut cleared: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

	for op in operations {
		match op {
			Operation::SetValue { key, value } => {
				overlay.insert(key, Some(value));
			}
			Operation::Clear { key } => {
				overlay.insert(key, None);
			}
			Operation::ClearRange { begin, end } => {
				for key in overlay_keys_in_range(&overlay, &begin, &end) {
					overlay.insert(key, None);
				}
				cleared.push((begin, end));
			}
			Operation::AtomicOp {
				key,
				param,
				op_type,
			} => {
				if matches!(op_type, MutationType::SetVersionstampedKey) {
					let key = substitute_raw_versionstamp(key, &versionstamp)
						.map_err(anyhow::Error::msg)
						.context("failed substituting versionstamped key")?;
					overlay.insert(key, Some(param));
					continue;
				}

				if matches!(op_type, MutationType::SetVersionstampedValue) {
					let value = substitute_raw_versionstamp(param, &versionstamp)
						.map_err(anyhow::Error::msg)
						.context("failed substituting versionstamped value")?;
					overlay.insert(key, Some(value));
					continue;
				}

				let current = match overlay.get(&key) {
					Some(value) => value.clone(),
					None if cleared_ranges_cover(&cleared, &key) => None,
					None => get_live(&db, &key).await?,
				};
				let new_value = apply_atomic_op(current.as_deref(), &param, op_type);
				overlay.insert(key, new_value);
			}
		}
	}

	for (begin, end) in &cleared {
		if begin >= end {
			continue;
		}
		let mut iter = db
			.scan_with_options(begin.clone()..end.clone(), &memory_scan_options())
			.await
			.context("failed to scan SlateDB clear range")?;
		while let Some(kv) = iter
			.next()
			.await
			.context("failed to iterate SlateDB clear range")?
		{
			let key = kv.key.to_vec();
			if !overlay.contains_key(&key) {
				overlay.insert(key, None);
			}
		}
	}

	let mut batch = WriteBatch::new();
	for (key, value) in overlay {
		if key.is_empty() {
			continue;
		}
		match value {
			Some(value) => batch.put(key, value),
			None => batch.delete(key),
		}
	}

	Ok(batch)
}
