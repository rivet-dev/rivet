//! UniversalDB helpers for sqlite-storage logical values.

use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, ensure};
use futures_util::TryStreamExt;
use universaldb::utils::{
	IsolationLevel::{Serializable, Snapshot},
	Subspace, end_of_key_range,
};

use crate::optimization_flags::{SqliteOptimizationFlags, sqlite_optimization_flags};

const CHUNK_KEY_PREFIX: u8 = 0x03;
const INLINE_VALUE_MARKER: u8 = 0x00;
const CHUNKED_VALUE_MARKER: u8 = 0x01;
const CHUNKED_METADATA_LEN: usize = 1 + std::mem::size_of::<u32>() + std::mem::size_of::<u32>();
const INLINE_VALUE_LIMIT: usize = 100_000;
pub const VALUE_CHUNK_SIZE: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOp {
	Put(Vec<u8>, Vec<u8>),
	Delete(Vec<u8>),
}

impl WriteOp {
	pub fn put(key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
		Self::Put(key.into(), value.into())
	}

	pub fn delete(key: impl Into<Vec<u8>>) -> Self {
		Self::Delete(key.into())
	}
}

pub async fn get_value(
	db: &universaldb::Database,
	subspace: &Subspace,
	op_counter: &AtomicUsize,
	key: Vec<u8>,
) -> Result<Option<Vec<u8>>> {
	run_db_op(db, op_counter, move |tx| {
		let subspace = subspace.clone();
		let key = key.clone();
		async move { tx_get_value(&tx, &subspace, &key).await }
	})
	.await
}

pub async fn batch_get_values(
	db: &universaldb::Database,
	subspace: &Subspace,
	op_counter: &AtomicUsize,
	keys: Vec<Vec<u8>>,
) -> Result<Vec<Option<Vec<u8>>>> {
	run_db_op(db, op_counter, move |tx| {
		let subspace = subspace.clone();
		let keys = keys.clone();
		async move {
			let mut values = Vec::with_capacity(keys.len());
			for key in &keys {
				values.push(tx_get_value(&tx, &subspace, key).await?);
			}

			Ok(values)
		}
	})
	.await
}

pub async fn scan_prefix_values(
	db: &universaldb::Database,
	subspace: &Subspace,
	op_counter: &AtomicUsize,
	prefix: Vec<u8>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	run_db_op(db, op_counter, move |tx| {
		let subspace = subspace.clone();
		let prefix = prefix.clone();
		async move { tx_scan_prefix_values(&tx, &subspace, &prefix).await }
	})
	.await
}

pub async fn apply_write_ops(
	db: &universaldb::Database,
	subspace: &Subspace,
	op_counter: &AtomicUsize,
	ops: Vec<WriteOp>,
) -> Result<()> {
	run_db_op(db, op_counter, move |tx| {
		let subspace = subspace.clone();
		let ops = ops.clone();
		async move {
			for op in &ops {
				match op {
					WriteOp::Put(key, value) => tx_write_value(&tx, &subspace, &key, &value)?,
					WriteOp::Delete(key) => tx_delete_value(&tx, &subspace, &key),
				}
			}
			#[cfg(test)]
			test_hooks::maybe_fail_apply_write_ops(&ops)?;

			Ok(())
		}
	})
	.await
}

pub(crate) async fn run_db_op<F, Fut, T>(
	db: &universaldb::Database,
	op_counter: &AtomicUsize,
	f: F,
) -> Result<T>
where
	F: Fn(universaldb::RetryableTransaction) -> Fut + Send + Sync,
	Fut: std::future::Future<Output = Result<T>> + Send,
	T: Send + 'static,
{
	op_counter.fetch_add(1, Ordering::SeqCst);
	db.run(f).await
}

pub(crate) async fn tx_get_value(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	let Some(metadata) = tx.get(&physical_key(subspace, key), Snapshot).await? else {
		return Ok(None);
	};

	Ok(Some(
		decode_value(tx, subspace, key, metadata.as_slice()).await?,
	))
}

/// Like tx_get_value, but registers the key in the transaction's read conflict
/// range so concurrent writes to the same key by other transactions cause this
/// transaction to abort and retry.
///
/// Use this for reads whose result is used to make a decision that depends on
/// the value not having changed (e.g. fence checks on META). Snapshot reads do
/// NOT register conflict ranges, so two transactions can both read the same
/// value at snapshot, both write, and FDB silently accepts both writes with
/// last-write-wins semantics — rewinding state.
pub(crate) async fn tx_get_value_serializable(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
) -> Result<Option<Vec<u8>>> {
	let Some(metadata) = tx.get(&physical_key(subspace, key), Serializable).await? else {
		return Ok(None);
	};

	Ok(Some(
		decode_value(tx, subspace, key, metadata.as_slice()).await?,
	))
}

pub(crate) async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let subspace_prefix_len = subspace.bytes().len();
	let physical_prefix = physical_key(subspace, prefix);
	let physical_prefix_subspace =
		Subspace::from(universaldb::tuple::Subspace::from_bytes(physical_prefix));
	let mut stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			mode: universaldb::options::StreamingMode::WantAll,
			..(&physical_prefix_subspace).into()
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		let logical_key = entry
			.key()
			.get(subspace_prefix_len..)
			.context("range entry key missing sqlite-storage subspace prefix")?
			.to_vec();
		let logical_value = decode_value(tx, subspace, &logical_key, entry.value()).await?;
		rows.push((logical_key, logical_value));
	}

	Ok(rows)
}

pub(crate) async fn tx_delete_value_precise(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
) -> Result<()> {
	let metadata = tx.get(&physical_key(subspace, key), Snapshot).await?;
	tx.clear(&physical_key(subspace, key));

	if let Some(metadata) = metadata.as_ref() {
		match metadata.first().copied() {
			Some(INLINE_VALUE_MARKER) | None => {}
			Some(CHUNKED_VALUE_MARKER) => {
				ensure!(
					metadata.len() == CHUNKED_METADATA_LEN,
					"chunked metadata for key {:?} had invalid length {}",
					key,
					metadata.len()
				);
				let chunk_count = u32::from_be_bytes(
					metadata[5..9]
						.try_into()
						.expect("chunked metadata count bytes should be present"),
				);
				for chunk_idx in 0..chunk_count {
					tx.clear(&physical_key(subspace, &chunk_key(key, chunk_idx)));
				}
			}
			Some(other) => {
				return Err(anyhow::anyhow!(
					"unknown sqlite-storage value marker {other} for key {:?}",
					key
				));
			}
		}
	}

	let prefix = chunk_key_prefix(key);
	let physical_prefix = physical_key(subspace, &prefix);
	tx.clear_range(&physical_prefix, &end_of_key_range(&physical_prefix));

	Ok(())
}

pub(crate) fn tx_write_value(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
	value: &[u8],
) -> Result<()> {
	tx_delete_value(tx, subspace, key);

	if value.len() <= INLINE_VALUE_LIMIT {
		tx.set(&physical_key(subspace, key), &encode_inline(value));
		return Ok(());
	}

	let chunk_count = value.len().div_ceil(VALUE_CHUNK_SIZE);
	tx.set(
		&physical_key(subspace, key),
		&encode_chunked_metadata(value.len(), chunk_count)?,
	);
	for (chunk_idx, chunk) in value.chunks(VALUE_CHUNK_SIZE).enumerate() {
		tx.set(
			&physical_key(subspace, &chunk_key(key, chunk_idx as u32)),
			chunk,
		);
	}

	Ok(())
}

pub(crate) fn tx_delete_value(tx: &universaldb::Transaction, subspace: &Subspace, key: &[u8]) {
	tx.clear(&physical_key(subspace, key));
	let prefix = chunk_key_prefix(key);
	let physical_prefix = physical_key(subspace, &prefix);
	tx.clear_range(&physical_prefix, &end_of_key_range(&physical_prefix));
}

async fn decode_value(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
	metadata: &[u8],
) -> Result<Vec<u8>> {
	decode_value_with_flags(tx, subspace, key, metadata, sqlite_optimization_flags()).await
}

async fn decode_value_with_flags(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
	metadata: &[u8],
	flags: &SqliteOptimizationFlags,
) -> Result<Vec<u8>> {
	let Some(marker) = metadata.first().copied() else {
		return Ok(Vec::new());
	};

	match marker {
		INLINE_VALUE_MARKER => Ok(metadata[1..].to_vec()),
		CHUNKED_VALUE_MARKER => {
			ensure!(
				metadata.len() == CHUNKED_METADATA_LEN,
				"chunked metadata for key {:?} had invalid length {}",
				key,
				metadata.len()
			);

			let total_len = u32::from_be_bytes(
				metadata[1..5]
					.try_into()
					.expect("chunked metadata length bytes should be present"),
			) as usize;
			let chunk_count = u32::from_be_bytes(
				metadata[5..9]
					.try_into()
					.expect("chunked metadata count bytes should be present"),
			) as usize;

			let mut value = if flags.batch_chunk_reads {
				read_chunked_value_range(tx, subspace, key, total_len, chunk_count).await?
			} else {
				read_chunked_value_serial(tx, subspace, key, total_len, chunk_count).await?
			};
			value.truncate(total_len);

			Ok(value)
		}
		other => Err(anyhow::anyhow!(
			"unknown sqlite-storage value marker {other} for key {:?}",
			key
		)),
	}
}

async fn read_chunked_value_serial(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
	total_len: usize,
	chunk_count: usize,
) -> Result<Vec<u8>> {
	let mut value = Vec::with_capacity(total_len);
	for chunk_idx in 0..chunk_count {
		#[cfg(test)]
		test_hooks::record_chunk_point_get();
		let chunk = tx
			.get(
				&physical_key(subspace, &chunk_key(key, chunk_idx as u32)),
				Snapshot,
			)
			.await?
			.with_context(|| format!("missing chunk {chunk_idx} for key {:?}", key))?;
		value.extend_from_slice(chunk.as_slice());
	}

	Ok(value)
}

async fn read_chunked_value_range(
	tx: &universaldb::Transaction,
	subspace: &Subspace,
	key: &[u8],
	total_len: usize,
	chunk_count: usize,
) -> Result<Vec<u8>> {
	let chunk_prefix = chunk_key_prefix(key);
	let physical_prefix = physical_key(subspace, &chunk_prefix);
	let physical_prefix_subspace =
		Subspace::from(universaldb::tuple::Subspace::from_bytes(physical_prefix));
	#[cfg(test)]
	test_hooks::record_chunk_range_get();
	let mut stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			limit: Some(chunk_count),
			mode: universaldb::options::StreamingMode::WantAll,
			..(&physical_prefix_subspace).into()
		},
		Snapshot,
	);

	let mut value = Vec::with_capacity(total_len);
	for chunk_idx in 0..chunk_count {
		let entry = stream
			.try_next()
			.await?
			.with_context(|| format!("missing chunk {chunk_idx} for key {:?}", key))?;
		let expected_key = physical_key(subspace, &chunk_key(key, chunk_idx as u32));
		ensure!(
			entry.key() == expected_key.as_slice(),
			"missing chunk {chunk_idx} for key {:?}",
			key
		);
		value.extend_from_slice(entry.value());
	}

	Ok(value)
}

fn encode_inline(value: &[u8]) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(1 + value.len());
	encoded.push(INLINE_VALUE_MARKER);
	encoded.extend_from_slice(value);
	encoded
}

fn encode_chunked_metadata(total_len: usize, chunk_count: usize) -> Result<Vec<u8>> {
	let total_len = u32::try_from(total_len).context("chunked value exceeded u32 length")?;
	let chunk_count = u32::try_from(chunk_count).context("chunked value exceeded u32 chunks")?;

	let mut encoded = Vec::with_capacity(CHUNKED_METADATA_LEN);
	encoded.push(CHUNKED_VALUE_MARKER);
	encoded.extend_from_slice(&total_len.to_be_bytes());
	encoded.extend_from_slice(&chunk_count.to_be_bytes());
	Ok(encoded)
}

fn chunk_key_prefix(key: &[u8]) -> Vec<u8> {
	let mut prefix = Vec::with_capacity(1 + key.len());
	prefix.push(CHUNK_KEY_PREFIX);
	prefix.extend_from_slice(key);
	prefix
}

fn chunk_key(key: &[u8], chunk_idx: u32) -> Vec<u8> {
	let prefix = chunk_key_prefix(key);
	let mut chunk_key = Vec::with_capacity(prefix.len() + std::mem::size_of::<u32>());
	chunk_key.extend_from_slice(&prefix);
	chunk_key.extend_from_slice(&chunk_idx.to_be_bytes());
	chunk_key
}

fn physical_key(subspace: &Subspace, key: &[u8]) -> Vec<u8> {
	[subspace.bytes(), key].concat()
}

#[cfg(test)]
pub fn physical_chunk_key(subspace: &Subspace, key: &[u8], chunk_idx: u32) -> Vec<u8> {
	physical_key(subspace, &chunk_key(key, chunk_idx))
}

#[cfg(test)]
mod tests {
	use std::sync::{Arc, atomic::AtomicUsize};

	use anyhow::Result;
	use tempfile::Builder;
	use uuid::Uuid;

	use super::*;

	async fn setup_db() -> Result<(universaldb::Database, Subspace, Arc<AtomicUsize>)> {
		let path = Builder::new()
			.prefix("sqlite-storage-udb-")
			.tempdir()?
			.keep();
		let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
		let db = universaldb::Database::new(Arc::new(driver));
		let subspace = Subspace::new(&("sqlite-storage-udb", Uuid::new_v4().to_string()));

		Ok((db, subspace, Arc::new(AtomicUsize::new(0))))
	}

	async fn write_chunked_value(
		db: &universaldb::Database,
		subspace: &Subspace,
		op_counter: &AtomicUsize,
		key: Vec<u8>,
		value: Vec<u8>,
	) -> Result<()> {
		apply_write_ops(db, subspace, op_counter, vec![WriteOp::put(key, value)]).await
	}

	async fn read_with_flags(
		db: &universaldb::Database,
		subspace: &Subspace,
		key: Vec<u8>,
		flags: SqliteOptimizationFlags,
	) -> Result<Vec<u8>> {
		db.run(move |tx| {
			let subspace = subspace.clone();
			let key = key.clone();
			async move {
				let metadata = tx
					.get(&physical_key(&subspace, &key), Snapshot)
					.await?
					.context("chunked test value should exist")?;
				decode_value_with_flags(&tx, &subspace, &key, metadata.as_slice(), &flags).await
			}
		})
		.await
	}

	#[tokio::test]
	async fn chunked_value_reads_use_bounded_range_by_default() -> Result<()> {
		let (db, subspace, op_counter) = setup_db().await?;
		let key = b"large-source-blob".to_vec();
		let value = vec![0x5a; INLINE_VALUE_LIMIT + VALUE_CHUNK_SIZE * 3 + 123];

		write_chunked_value(&db, &subspace, op_counter.as_ref(), key.clone(), value.clone()).await?;
		test_hooks::reset_chunk_read_counts();
		let read = read_with_flags(&db, &subspace, key, SqliteOptimizationFlags::default()).await?;

		assert_eq!(read, value);
		assert_eq!(test_hooks::chunk_range_get_count(), 1);
		assert_eq!(test_hooks::chunk_point_get_count(), 0);

		Ok(())
	}

	#[tokio::test]
	async fn disabled_batch_chunk_reads_use_compatible_serial_chunks() -> Result<()> {
		let (db, subspace, op_counter) = setup_db().await?;
		let key = b"legacy-large-source-blob".to_vec();
		let value = vec![0x42; INLINE_VALUE_LIMIT + VALUE_CHUNK_SIZE * 2 + 1];
		let chunk_count = value.len().div_ceil(VALUE_CHUNK_SIZE);
		let flags = SqliteOptimizationFlags {
			batch_chunk_reads: false,
			..SqliteOptimizationFlags::default()
		};

		write_chunked_value(&db, &subspace, op_counter.as_ref(), key.clone(), value.clone()).await?;
		test_hooks::reset_chunk_read_counts();
		let read = read_with_flags(&db, &subspace, key, flags).await?;

		assert_eq!(read, value);
		assert_eq!(test_hooks::chunk_range_get_count(), 0);
		assert_eq!(test_hooks::chunk_point_get_count(), chunk_count);

		Ok(())
	}
}

#[cfg(test)]
pub async fn raw_key_exists(
	db: &universaldb::Database,
	op_counter: &AtomicUsize,
	key: Vec<u8>,
) -> Result<bool> {
	run_db_op(db, op_counter, move |tx| {
		let key = key.clone();
		async move { Ok(tx.get(&key, Snapshot).await?.is_some()) }
	})
	.await
}

#[cfg(test)]
pub mod test_hooks {
	use std::sync::{
		Mutex,
		atomic::{AtomicUsize, Ordering},
	};

	use anyhow::{Result, bail};

	use crate::udb::WriteOp;

	static FAIL_NEXT_APPLY_WRITE_OPS_PREFIX: Mutex<Option<Vec<u8>>> = Mutex::new(None);
	static CHUNK_POINT_GETS: AtomicUsize = AtomicUsize::new(0);
	static CHUNK_RANGE_GETS: AtomicUsize = AtomicUsize::new(0);

	pub struct ApplyWriteOpsFailureGuard;

	pub fn fail_next_apply_write_ops_matching(prefix: Vec<u8>) -> ApplyWriteOpsFailureGuard {
		*FAIL_NEXT_APPLY_WRITE_OPS_PREFIX
			.lock()
			.expect("apply_write_ops failpoint mutex should lock") = Some(prefix);
		ApplyWriteOpsFailureGuard
	}

	pub(crate) fn maybe_fail_apply_write_ops(ops: &[WriteOp]) -> Result<()> {
		let mut fail_prefix = FAIL_NEXT_APPLY_WRITE_OPS_PREFIX
			.lock()
			.expect("apply_write_ops failpoint mutex should lock");
		let should_fail = fail_prefix.as_ref().is_some_and(|prefix| {
			ops.iter().any(|op| match op {
				WriteOp::Put(key, _) | WriteOp::Delete(key) => key.starts_with(prefix),
			})
		});
		if should_fail {
			*fail_prefix = None;
			bail!("InjectedStoreError: apply_write_ops failed before commit");
		}

		Ok(())
	}

	pub(crate) fn record_chunk_point_get() {
		CHUNK_POINT_GETS.fetch_add(1, Ordering::SeqCst);
	}

	pub(crate) fn record_chunk_range_get() {
		CHUNK_RANGE_GETS.fetch_add(1, Ordering::SeqCst);
	}

	pub fn reset_chunk_read_counts() {
		CHUNK_POINT_GETS.store(0, Ordering::SeqCst);
		CHUNK_RANGE_GETS.store(0, Ordering::SeqCst);
	}

	pub fn chunk_point_get_count() -> usize {
		CHUNK_POINT_GETS.load(Ordering::SeqCst)
	}

	pub fn chunk_range_get_count() -> usize {
		CHUNK_RANGE_GETS.load(Ordering::SeqCst)
	}

	impl Drop for ApplyWriteOpsFailureGuard {
		fn drop(&mut self) {
			*FAIL_NEXT_APPLY_WRITE_OPS_PREFIX
				.lock()
				.expect("apply_write_ops failpoint mutex should lock") = None;
		}
	}
}

#[cfg(test)]
pub fn op_count(counter: &std::sync::Arc<AtomicUsize>) -> usize {
	counter.load(Ordering::SeqCst)
}

#[cfg(test)]
pub fn clear_op_count(counter: &std::sync::Arc<AtomicUsize>) {
	counter.store(0, Ordering::SeqCst);
}
