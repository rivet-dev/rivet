use std::sync::Arc;

use anyhow::{Context, Result, bail};
use rocksdb::{
	OptimisticTransactionDB, ReadOptions, Transaction as RocksDbTransaction, WriteOptions,
};
use tokio::sync::{mpsc, oneshot};

use super::transaction_conflict_tracker::TransactionConflictTracker;
use crate::{
	atomic::apply_atomic_op,
	error::DatabaseError,
	key_selector::KeySelector,
	options::ConflictRangeType,
	tx_ops::Operation,
	value::{KeyValue, Slice, Values},
	versionstamp::substitute_versionstamp_if_incomplete,
};

pub enum TransactionCommand {
	Get {
		key: Vec<u8>,
		response: oneshot::Sender<Result<Option<Slice>>>,
	},
	GetKey {
		key: Vec<u8>,
		or_equal: bool,
		offset: i32,
		response: oneshot::Sender<Result<Option<Slice>>>,
	},
	GetRange {
		begin: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		reverse: bool,
		response: oneshot::Sender<Result<Values>>,
	},
	Commit {
		start_version: u64,
		operations: Vec<Operation>,
		conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
		response: oneshot::Sender<Result<()>>,
	},
	GetEstimatedRangeSize {
		begin: Vec<u8>,
		end: Vec<u8>,
		response: oneshot::Sender<Result<i64>>,
	},
}

// This task may be used for multiple rocksdb txns, in contrast to how postgres is written. This is solely to
// save on spawning new tasks.
pub struct TransactionTask {
	db: Arc<OptimisticTransactionDB>,
	txn_conflict_tracker: TransactionConflictTracker,
	receiver: mpsc::Receiver<TransactionCommand>,
}

impl TransactionTask {
	pub fn new(
		db: Arc<OptimisticTransactionDB>,
		txn_conflict_tracker: TransactionConflictTracker,
		receiver: mpsc::Receiver<TransactionCommand>,
	) -> Self {
		TransactionTask {
			db,
			txn_conflict_tracker,
			receiver,
		}
	}

	pub async fn run(mut self) {
		while let Some(command) = self.receiver.recv().await {
			match command {
				TransactionCommand::Get { key, response } => {
					let result = self.handle_get(&key).await;
					let _ = response.send(result);
				}
				TransactionCommand::GetKey {
					key,
					or_equal,
					offset,
					response,
				} => {
					let result = self.handle_get_key(&key, or_equal, offset).await;
					let _ = response.send(result);
				}
				TransactionCommand::GetRange {
					begin,
					begin_or_equal,
					begin_offset,
					end,
					end_or_equal,
					end_offset,
					limit,
					reverse,
					response,
				} => {
					let result = self
						.handle_get_range(
							begin,
							begin_or_equal,
							begin_offset,
							end,
							end_or_equal,
							end_offset,
							limit,
							reverse,
						)
						.await;
					let _ = response.send(result);
				}
				TransactionCommand::Commit {
					start_version,
					operations,
					conflict_ranges,
					response,
				} => {
					let result = self
						.handle_commit(start_version, operations, conflict_ranges)
						.await;
					let _ = response.send(result);
				}
				TransactionCommand::GetEstimatedRangeSize {
					begin,
					end,
					response,
				} => {
					let result = self.handle_get_estimated_range_size(&begin, &end).await;
					let _ = response.send(result);
				}
			}
		}
	}

	fn create_transaction(&self) -> RocksDbTransaction<'_, OptimisticTransactionDB> {
		let write_opts = WriteOptions::default();
		let mut txn_opts = rocksdb::OptimisticTransactionOptions::default();
		txn_opts.set_snapshot(true);
		self.db.transaction_opt(&write_opts, &txn_opts)
	}

	async fn handle_get(&mut self, key: &[u8]) -> Result<Option<Slice>> {
		let txn = self.create_transaction();

		let read_opts = ReadOptions::default();

		Ok(txn
			.get_opt(key, &read_opts)
			.context("failed to read key from rocksdb")?
			.map(|v| v.into()))
	}

	async fn handle_get_key(
		&mut self,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> Result<Option<Slice>> {
		let txn = self.create_transaction();

		let read_opts = ReadOptions::default();

		// Based on PostgreSQL's interpretation:
		// (false, 1) => first_greater_or_equal
		// (true, 1) => first_greater_than
		// (false, 0) => last_less_than
		// (true, 0) => last_less_or_equal

		match (or_equal, offset) {
			(false, 1) => {
				// first_greater_or_equal: find first key >= search_key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
					read_opts,
				);
				for item in iter {
					let (k, _v) =
						item.context("failed to iterate rocksdb for first_greater_or_equal")?;
					return Ok(Some(k.to_vec().into()));
				}
				Ok(None)
			}
			(true, 1) => {
				// first_greater_than: find first key > search_key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
					read_opts,
				);
				for item in iter {
					let (k, _v) =
						item.context("failed to iterate rocksdb for first_greater_than")?;
					// Skip if it's the exact key
					if k.as_ref() == key {
						continue;
					}
					return Ok(Some(k.to_vec().into()));
				}
				Ok(None)
			}
			(false, 0) => {
				// last_less_than: find last key < search_key
				// Use reverse iterator starting just before the key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Reverse),
					read_opts,
				);

				for item in iter {
					let (k, _v) = item.context("failed to iterate rocksdb for last_less_than")?;
					// We want strictly less than
					if k.as_ref() < key {
						return Ok(Some(k.to_vec().into()));
					}
				}
				Ok(None)
			}
			(true, 0) => {
				// last_less_or_equal: find last key <= search_key
				// Use reverse iterator starting from the key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Reverse),
					read_opts,
				);

				for item in iter {
					let (k, _v) =
						item.context("failed to iterate rocksdb for last_less_or_equal")?;
					// We want less than or equal
					if k.as_ref() <= key {
						return Ok(Some(k.to_vec().into()));
					}
				}
				Ok(None)
			}
			_ => {
				// For other offset values, return an error
				bail!("invalid key selector offset")
			}
		}
	}

	#[allow(dead_code)]
	fn resolve_key_selector(
		&self,
		txn: &RocksDbTransaction<OptimisticTransactionDB>,
		selector: &KeySelector<'_>,
		_read_opts: &ReadOptions,
	) -> Result<Vec<u8>> {
		let key = selector.key();
		let offset = selector.offset();
		let or_equal = selector.or_equal();

		if offset == 0 && or_equal {
			// Simple case: exact key
			return Ok(key.to_vec());
		}

		// Create an iterator to find the key
		let iter = txn.iterator_opt(
			rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
			ReadOptions::default(),
		);

		let mut keys: Vec<Vec<u8>> = Vec::new();

		for item in iter {
			let (k, _v) = item.context("failed to iterate rocksdb for key selector")?;
			keys.push(k.to_vec());
			if keys.len() > (offset.abs() + 1) as usize {
				break;
			}
		}

		// Apply the selector logic
		let idx = if or_equal {
			// If or_equal is true and the key exists, use it
			if !keys.is_empty() && keys[0] == key {
				offset.max(0) as usize
			} else {
				// Otherwise, use the next key
				if offset >= 0 {
					offset as usize
				} else {
					return Ok(Vec::new());
				}
			}
		} else {
			// If or_equal is false, skip the exact match
			let skip = if !keys.is_empty() && keys[0] == key {
				1
			} else {
				0
			};
			(skip + offset.max(0)) as usize
		};

		if idx < keys.len() {
			Ok(keys[idx].clone())
		} else {
			Ok(Vec::new())
		}
	}

	async fn handle_commit(
		&mut self,
		start_version: u64,
		operations: Vec<Operation>,
		conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	) -> Result<()> {
		// Create a new transaction for this commit
		let txn = self.create_transaction();

		// Apply all operations to the transaction
		for op in operations {
			match op {
				Operation::Set { key, value } => {
					// Substitute versionstamp if incomplete
					// For now, just use the simple substitution - we can improve this later
					// to ensure all versionstamps in a transaction have the same base timestamp
					let value = substitute_versionstamp_if_incomplete(value.clone(), 0);

					txn.put(key, &value)
						.context("failed to set key in rocksdb")?;
				}
				Operation::Clear { key } => {
					txn.delete(key)
						.context("failed to delete key from rocksdb")?;
				}
				Operation::ClearRange { begin, end } => {
					// RocksDB doesn't have a native clear_range, so we need to iterate and delete
					let read_opts = ReadOptions::default();
					let iter = txn.iterator_opt(
						rocksdb::IteratorMode::From(&begin, rocksdb::Direction::Forward),
						read_opts,
					);

					for item in iter {
						let (k, _v) = item.context("failed to iterate rocksdb for clear range")?;
						if k.as_ref() >= end.as_slice() {
							break;
						}
						txn.delete(&k)
							.context("failed to delete key in range from rocksdb")?;
					}
				}
				Operation::AtomicOp {
					key,
					param,
					op_type,
				} => {
					// Get the current value from the database
					let read_opts = ReadOptions::default();
					let current_value = txn
						.get_opt(&key, &read_opts)
						.context("failed to get current value for atomic operation")?;

					// Apply the atomic operation
					let current_slice = current_value.as_deref();
					let new_value = apply_atomic_op(current_slice, &param, op_type);

					// Store the result
					if let Some(new_value) = &new_value {
						txn.put(key, new_value)
							.context("failed to set atomic operation result")?;
					} else {
						txn.delete(key)
							.context("failed to delete key after atomic operation")?;
					}
				}
			}
		}

		if self
			.txn_conflict_tracker
			.check_and_insert(start_version, conflict_ranges)
			.await
		{
			return Err(DatabaseError::NotCommitted.into());
		}

		// Commit the transaction (this consumes txn)
		match txn.commit() {
			Ok(_) => Ok(()),
			Err(e) => {
				// If the txn failed due to a rocksdb error, remove it from the conflict tracker
				self.txn_conflict_tracker.remove(start_version).await;

				let err_str = e.to_string();

				// Check if this is a conflict error
				if err_str.contains("conflict") || err_str.contains("Resource busy") {
					// Return retryable error
					Err(DatabaseError::NotCommitted.into())
				} else {
					Err(e).context("rocksdb commit error")
				}
			}
		}
	}

	async fn handle_get_range(
		&mut self,
		begin: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		reverse: bool,
	) -> Result<Values> {
		let txn = self.create_transaction();
		let read_opts = ReadOptions::default();

		// Resolve the begin selector
		let resolved_begin =
			self.resolve_key_selector_for_range(&txn, &begin, begin_or_equal, begin_offset)?;

		// Resolve the end selector
		let resolved_end =
			self.resolve_key_selector_for_range(&txn, &end, end_or_equal, end_offset)?;

		// Now execute the range query with resolved keys
		let iter = txn.iterator_opt(
			rocksdb::IteratorMode::From(&resolved_begin, rocksdb::Direction::Forward),
			read_opts,
		);

		let mut results = Vec::new();
		let limit = limit.unwrap_or(usize::MAX);

		for item in iter {
			let (k, v) = item.context("failed to iterate rocksdb for get range")?;
			// Check if we've reached the end key
			if k.as_ref() >= resolved_end.as_slice() {
				break;
			}

			results.push(KeyValue::new(k.to_vec(), v.to_vec()));

			if results.len() >= limit {
				break;
			}
		}

		// Apply reverse if needed
		if reverse {
			results.reverse();
		}

		Ok(Values::new(results))
	}

	fn resolve_key_selector_for_range(
		&self,
		txn: &RocksDbTransaction<OptimisticTransactionDB>,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> Result<Vec<u8>> {
		// Based on PostgreSQL's interpretation:
		// (false, 1) => first_greater_or_equal
		// (true, 1) => first_greater_than
		// (false, 0) => last_less_than
		// (true, 0) => last_less_or_equal

		let read_opts = ReadOptions::default();

		match (or_equal, offset) {
			(false, 1) => {
				// first_greater_or_equal: find first key >= search_key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
					read_opts,
				);
				for item in iter {
					let (k, _v) = item.context(
						"failed to iterate rocksdb for range selector first_greater_or_equal",
					)?;
					return Ok(k.to_vec());
				}
				// If no key found, return a key that will make the range empty
				Ok(vec![0xff; 255])
			}
			(true, 1) => {
				// first_greater_than: find first key > search_key
				let iter = txn.iterator_opt(
					rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
					read_opts,
				);
				for item in iter {
					let (k, _v) = item.context(
						"failed to iterate rocksdb for range selector first_greater_than",
					)?;
					// Skip if it's the exact key
					if k.as_ref() == key {
						continue;
					}
					return Ok(k.to_vec());
				}
				// If no key found, return a key that will make the range empty
				Ok(vec![0xff; 255])
			}
			_ => {
				// For other cases, just use the key as-is for now
				// This is a simplification - full implementation would handle all cases
				Ok(key.to_vec())
			}
		}
	}

	async fn handle_get_estimated_range_size(&mut self, begin: &[u8], end: &[u8]) -> Result<i64> {
		let range = rocksdb::Range::new(begin, end);

		Ok(self
			.db
			.get_approximate_sizes(&[range])
			.first()
			.copied()
			.unwrap_or(0) as i64)
	}
}
