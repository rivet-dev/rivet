use std::sync::Arc;

use rocksdb::{
	OptimisticTransactionDB, ReadOptions, Transaction as RocksDbTransaction, WriteOptions,
};
use tokio::sync::{mpsc, oneshot};

use crate::{
	FdbError, FdbResult, KeySelector, TransactionCommitted,
	atomic::apply_atomic_op,
	future::{FdbKeyValue, FdbSlice, FdbValues},
	tx_ops::{Operation, TransactionOperations},
};

pub enum TransactionCommand {
	Get {
		key: Vec<u8>,
		response: oneshot::Sender<FdbResult<Option<FdbSlice>>>,
	},
	GetKey {
		key: Vec<u8>,
		or_equal: bool,
		offset: i32,
		response: oneshot::Sender<FdbResult<Option<FdbSlice>>>,
	},
	GetRange {
		begin_key: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end_key: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		reverse: bool,
		iteration: usize,
		response: oneshot::Sender<FdbResult<FdbValues>>,
	},
	Commit {
		operations: TransactionOperations,
		response: oneshot::Sender<FdbResult<TransactionCommitted>>,
	},
	GetEstimatedRangeSize {
		begin: Vec<u8>,
		end: Vec<u8>,
		response: oneshot::Sender<FdbResult<i64>>,
	},
	Cancel,
}

pub struct TransactionTask {
	db: Arc<OptimisticTransactionDB>,
	receiver: mpsc::Receiver<TransactionCommand>,
	_exclusive: bool,
}

impl TransactionTask {
	pub fn new(
		db: Arc<OptimisticTransactionDB>,
		receiver: mpsc::Receiver<TransactionCommand>,
		exclusive: bool,
	) -> Self {
		TransactionTask {
			db,
			receiver,
			_exclusive: exclusive,
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
					begin_key,
					begin_or_equal,
					begin_offset,
					end_key,
					end_or_equal,
					end_offset,
					limit,
					reverse,
					iteration,
					response,
				} => {
					let result = self
						.handle_get_range(
							begin_key,
							begin_or_equal,
							begin_offset,
							end_key,
							end_or_equal,
							end_offset,
							limit,
							reverse,
							iteration,
						)
						.await;
					let _ = response.send(result);
				}
				TransactionCommand::Commit {
					operations,
					response,
				} => {
					let result = self.handle_commit(operations).await;
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
				TransactionCommand::Cancel => {
					// Exit the task
					break;
				}
			}
		}
	}

	fn create_transaction(&self) -> RocksDbTransaction<OptimisticTransactionDB> {
		let write_opts = WriteOptions::default();
		let txn_opts = rocksdb::OptimisticTransactionOptions::default();
		self.db.transaction_opt(&write_opts, &txn_opts)
	}

	async fn handle_get(&mut self, key: &[u8]) -> FdbResult<Option<FdbSlice>> {
		let txn = self.create_transaction();

		let read_opts = ReadOptions::default();

		match txn.get_opt(key, &read_opts) {
			Ok(Some(value)) => Ok(Some(value)),
			Ok(None) => Ok(None),
			Err(_) => Err(FdbError::from_code(1510)),
		}
	}

	async fn handle_get_key(
		&mut self,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> FdbResult<Option<FdbSlice>> {
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
					match item {
						Ok((k, _v)) => {
							return Ok(Some(k.to_vec()));
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
					}
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
					match item {
						Ok((k, _v)) => {
							// Skip if it's the exact key
							if k.as_ref() == key {
								continue;
							}
							return Ok(Some(k.to_vec()));
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
					}
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
					match item {
						Ok((k, _v)) => {
							// We want strictly less than
							if k.as_ref() < key {
								return Ok(Some(k.to_vec()));
							}
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
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
					match item {
						Ok((k, _v)) => {
							// We want less than or equal
							if k.as_ref() <= key {
								return Ok(Some(k.to_vec()));
							}
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
					}
				}
				Ok(None)
			}
			_ => {
				// For other offset values, return an error
				Err(FdbError::from_code(1510))
			}
		}
	}

	#[allow(dead_code)]
	fn resolve_key_selector(
		&self,
		txn: &RocksDbTransaction<OptimisticTransactionDB>,
		selector: &KeySelector<'_>,
		_read_opts: &ReadOptions,
	) -> FdbResult<Vec<u8>> {
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
			match item {
				Ok((k, _v)) => {
					keys.push(k.to_vec());
					if keys.len() > (offset.abs() + 1) as usize {
						break;
					}
				}
				Err(_) => return Err(FdbError::from_code(1510)),
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
		operations: TransactionOperations,
	) -> FdbResult<TransactionCommitted> {
		// Create a new transaction for this commit
		let txn = self.create_transaction();

		// Apply all operations to the transaction
		for op in operations.operations() {
			match op {
				Operation::Set { key, value } => {
					// Substitute versionstamp if incomplete
					// For now, just use the simple substitution - we can improve this later
					// to ensure all versionstamps in a transaction have the same base timestamp
					let value = crate::versionstamp::substitute_versionstamp_if_incomplete(
						value.clone(),
						0,
					);

					txn.put(key, &value)
						.map_err(|_| FdbError::from_code(1510))?;
				}
				Operation::Clear { key } => {
					txn.delete(key).map_err(|_| FdbError::from_code(1510))?;
				}
				Operation::ClearRange { begin, end } => {
					// RocksDB doesn't have a native clear_range, so we need to iterate and delete
					let read_opts = ReadOptions::default();
					let iter = txn.iterator_opt(
						rocksdb::IteratorMode::From(begin, rocksdb::Direction::Forward),
						read_opts,
					);

					for item in iter {
						match item {
							Ok((k, _v)) => {
								if k.as_ref() >= end.as_slice() {
									break;
								}
								txn.delete(&k).map_err(|_| FdbError::from_code(1510))?;
							}
							Err(_) => return Err(FdbError::from_code(1510)),
						}
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
						.get_opt(key, &read_opts)
						.map_err(|_| FdbError::from_code(1510))?;

					// Apply the atomic operation
					let current_slice = current_value.as_deref();
					let new_value = apply_atomic_op(current_slice, param, *op_type);

					// Store the result
					if let Some(new_value) = &new_value {
						txn.put(key, new_value)
							.map_err(|_| FdbError::from_code(1510))?;
					} else {
						txn.delete(key).map_err(|_| FdbError::from_code(1510))?;
					}
				}
			}
		}

		// Note: RocksDB doesn't natively support conflict ranges like FoundationDB
		// We would need to implement custom conflict detection here if needed
		// For now, we'll rely on OptimisticTransactionDB's built-in conflict detection

		// Commit the transaction (this consumes txn)
		match txn.commit() {
			Ok(_) => Ok(()),
			Err(e) => {
				// Check if this is a conflict error
				if e.to_string().contains("conflict") {
					// Return retryable error code 1020
					Err(FdbError::from_code(1020))
				} else {
					Err(FdbError::from_code(1510))
				}
			}
		}
	}

	async fn handle_get_range(
		&mut self,
		begin_key: Vec<u8>,
		begin_or_equal: bool,
		begin_offset: i32,
		end_key: Vec<u8>,
		end_or_equal: bool,
		end_offset: i32,
		limit: Option<usize>,
		reverse: bool,
		_iteration: usize,
	) -> FdbResult<FdbValues> {
		let txn = self.create_transaction();
		let read_opts = ReadOptions::default();

		// Resolve the begin selector
		let resolved_begin =
			self.resolve_key_selector_for_range(&txn, &begin_key, begin_or_equal, begin_offset)?;

		// Resolve the end selector
		let resolved_end =
			self.resolve_key_selector_for_range(&txn, &end_key, end_or_equal, end_offset)?;

		// Now execute the range query with resolved keys
		let iter = txn.iterator_opt(
			rocksdb::IteratorMode::From(&resolved_begin, rocksdb::Direction::Forward),
			read_opts,
		);

		let mut results = Vec::new();
		let limit = limit.unwrap_or(usize::MAX);

		for item in iter {
			match item {
				Ok((k, v)) => {
					// Check if we've reached the end key
					if k.as_ref() >= resolved_end.as_slice() {
						break;
					}

					results.push(FdbKeyValue::new(k.to_vec(), v.to_vec()));

					if results.len() >= limit {
						break;
					}
				}
				Err(_) => return Err(FdbError::from_code(1510)),
			}
		}

		// Apply reverse if needed
		if reverse {
			results.reverse();
		}

		Ok(FdbValues::new(results))
	}

	fn resolve_key_selector_for_range(
		&self,
		txn: &RocksDbTransaction<OptimisticTransactionDB>,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> FdbResult<Vec<u8>> {
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
					match item {
						Ok((k, _v)) => {
							return Ok(k.to_vec());
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
					}
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
					match item {
						Ok((k, _v)) => {
							// Skip if it's the exact key
							if k.as_ref() == key {
								continue;
							}
							return Ok(k.to_vec());
						}
						Err(_) => {
							return Err(FdbError::from_code(1510));
						}
					}
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

	async fn handle_get_estimated_range_size(
		&mut self,
		begin: &[u8],
		end: &[u8],
	) -> FdbResult<i64> {
		let range = rocksdb::Range::new(begin, end);

		Ok(self
			.db
			.get_approximate_sizes(&[range])
			.first()
			.copied()
			.unwrap_or(0) as i64)
	}
}
