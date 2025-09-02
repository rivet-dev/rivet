use std::{
	future::Future,
	pin::Pin,
	sync::{Arc, Mutex},
};

use rocksdb::OptimisticTransactionDB;
use tokio::sync::{OnceCell, mpsc, oneshot};

use crate::{
	FdbError, FdbResult, KeySelector, RangeOption, TransactionCommitError, TransactionCommitted,
	driver::TransactionDriver,
	future::{FdbSlice, FdbValues},
	options::{ConflictRangeType, MutationType},
	tx_ops::TransactionOperations,
};

use super::{
	conflict_range_tracker::{ConflictRangeTracker, TransactionId},
	transaction_task::{TransactionCommand, TransactionTask},
};

struct TransactionState {
	operations: TransactionOperations,
	committed: bool,
}

impl Default for TransactionState {
	fn default() -> Self {
		Self {
			operations: TransactionOperations::default(),
			committed: false,
		}
	}
}

pub struct RocksDbTransactionDriver {
	db: Arc<OptimisticTransactionDB>,
	state: Arc<Mutex<TransactionState>>,
	tx_sender: Arc<OnceCell<mpsc::Sender<TransactionCommand>>>,
	snapshot_tx_sender: Arc<OnceCell<mpsc::Sender<TransactionCommand>>>,
	conflict_tracker: ConflictRangeTracker,
	tx_id: TransactionId,
}

impl Drop for RocksDbTransactionDriver {
	fn drop(&mut self) {
		// Release all conflict ranges when the transaction is dropped
		self.conflict_tracker.release_transaction(self.tx_id);
	}
}

impl RocksDbTransactionDriver {
	pub fn new(db: Arc<OptimisticTransactionDB>, conflict_tracker: ConflictRangeTracker) -> Self {
		RocksDbTransactionDriver {
			db,
			state: Arc::new(Mutex::new(TransactionState::default())),
			tx_sender: Arc::new(OnceCell::new()),
			snapshot_tx_sender: Arc::new(OnceCell::new()),
			conflict_tracker,
			tx_id: TransactionId::new(),
		}
	}

	/// Get or create the transaction task for non-snapshot operations
	async fn ensure_transaction(&self) -> FdbResult<&mpsc::Sender<TransactionCommand>> {
		self.tx_sender
			.get_or_try_init(|| async {
				let (sender, receiver) = mpsc::channel(100);

				// Spawn the transaction task
				let task = TransactionTask::new(
					self.db.clone(),
					receiver,
					true, // exclusive = true for non-snapshot reads
				);
				tokio::spawn(task.run());

				Ok(sender)
			})
			.await
			.map_err(|_: anyhow::Error| FdbError::from_code(1510))
	}

	/// Get or create the transaction task for snapshot operations
	async fn ensure_snapshot_transaction(&self) -> FdbResult<&mpsc::Sender<TransactionCommand>> {
		self.snapshot_tx_sender
			.get_or_try_init(|| async {
				let (sender, receiver) = mpsc::channel(100);

				// Spawn the transaction task
				let task = TransactionTask::new(
					self.db.clone(),
					receiver,
					false, // exclusive = false for snapshot reads
				);
				tokio::spawn(task.run());

				Ok(sender)
			})
			.await
			.map_err(|_: anyhow::Error| FdbError::from_code(1510))
	}
}

impl TransactionDriver for RocksDbTransactionDriver {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		// Add write conflict range for this key
		let _ = self.conflict_tracker.add_range(
			self.tx_id,
			key,
			&[key, &[0u8]].concat(), // Key range is [key, key+\0)
			true,                    // is_write = true for atomic operations
		);

		let mut state = self.state.lock().unwrap();
		state.operations.atomic_op(key, param, op_type);
	}

	fn get<'a>(
		&'a self,
		key: &[u8],
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<Option<FdbSlice>>> + Send + 'a>> {
		let key = key.to_vec();
		Box::pin(async move {
			// Both snapshot and non-snapshot reads check local operations first
			// Transactions always see their own writes
			let ops = {
				let state = self.state.lock().unwrap();
				state.operations.clone()
			};

			ops.get_with_callback(&key, || async {
				if snapshot {
					// For snapshot reads, don't add conflict ranges
					let tx_sender = self.ensure_snapshot_transaction().await?;

					// Send query command
					let (response_tx, response_rx) = oneshot::channel();
					tx_sender
						.send(TransactionCommand::Get {
							key: key.clone(),
							response: response_tx,
						})
						.await
						.map_err(|_| FdbError::from_code(1510))?;

					// Wait for response
					let value = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

					Ok(value)
				} else {
					// Add read conflict range for this key
					self.conflict_tracker.add_range(
						self.tx_id,
						&key,
						&[&key[..], &[0u8]].concat(), // Key range is [key, key+\0)
						false,                        // is_write = false for reads
					)?;

					let tx_sender = self.ensure_transaction().await?;

					// Send query command
					let (response_tx, response_rx) = oneshot::channel();
					tx_sender
						.send(TransactionCommand::Get {
							key: key.clone(),
							response: response_tx,
						})
						.await
						.map_err(|_| FdbError::from_code(1510))?;

					// Wait for response
					let value = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

					Ok(value)
				}
			})
			.await
		})
	}

	fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<FdbSlice>> + Send + 'a>> {
		let selector = selector.clone();

		Box::pin(async move {
			let key = selector.key().to_vec();
			let offset = selector.offset();
			let or_equal = selector.or_equal();

			// Both snapshot and non-snapshot reads check local operations first
			// Transactions always see their own writes
			let ops = {
				let state = self.state.lock().unwrap();
				state.operations.clone()
			};

			ops.get_key(&selector, || async {
				let tx_sender = if snapshot {
					self.ensure_snapshot_transaction().await?
				} else {
					self.ensure_transaction().await?
				};

				// Send query command
				let (response_tx, response_rx) = oneshot::channel();
				tx_sender
					.send(TransactionCommand::GetKey {
						key: key.clone(),
						or_equal,
						offset,
						response: response_tx,
					})
					.await
					.map_err(|_| FdbError::from_code(1510))?;

				// Wait for response
				let result_key = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

				// Return the key if found, or empty vector if not
				Ok(result_key.unwrap_or_else(Vec::new))
			})
			.await
		})
	}

	fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<FdbValues>> + Send + 'a>> {
		// Extract fields from RangeOption for the async closure
		let opt = opt.clone();
		let begin_selector = opt.begin.clone();
		let end_selector = opt.end.clone();
		let limit = opt.limit;
		let reverse = opt.reverse;

		Box::pin(async move {
			// Both snapshot and non-snapshot reads check local operations first
			// Transactions always see their own writes
			let ops = {
				let state = self.state.lock().unwrap();
				state.operations.clone()
			};

			ops.get_range(&opt, || async {
				if snapshot {
					// For snapshot reads, don't add conflict ranges
					let tx_sender = self.ensure_snapshot_transaction().await?;

					// Send query command with selector info
					let (response_tx, response_rx) = oneshot::channel();
					tx_sender
						.send(TransactionCommand::GetRange {
							begin_key: begin_selector.key().to_vec(),
							begin_or_equal: begin_selector.or_equal(),
							begin_offset: begin_selector.offset(),
							end_key: end_selector.key().to_vec(),
							end_or_equal: end_selector.or_equal(),
							end_offset: end_selector.offset(),
							limit,
							reverse,
							iteration,
							response: response_tx,
						})
						.await
						.map_err(|_| FdbError::from_code(1510))?;

					// Wait for response
					let values = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

					Ok(values)
				} else {
					// Add read conflict range for this range (using raw keys, conservative)
					self.conflict_tracker.add_range(
						self.tx_id,
						begin_selector.key(),
						end_selector.key(),
						false, // is_write = false for reads
					)?;

					let tx_sender = self.ensure_transaction().await?;

					// Send query command with selector info
					let (response_tx, response_rx) = oneshot::channel();
					tx_sender
						.send(TransactionCommand::GetRange {
							begin_key: begin_selector.key().to_vec(),
							begin_or_equal: begin_selector.or_equal(),
							begin_offset: begin_selector.offset(),
							end_key: end_selector.key().to_vec(),
							end_or_equal: end_selector.or_equal(),
							end_offset: end_selector.offset(),
							limit,
							reverse,
							iteration,
							response: response_tx,
						})
						.await
						.map_err(|_| FdbError::from_code(1510))?;

					// Wait for response
					let values = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

					Ok(values)
				}
			})
			.await
		})
	}

	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		snapshot: bool,
	) -> crate::future::FdbStream<'a, crate::future::FdbValue> {
		use futures_util::StreamExt;

		// Extract the selectors from RangeOption, same as get_range does
		let begin_selector = opt.begin.clone();
		let end_selector = opt.end.clone();
		let limit = opt.limit;
		let reverse = opt.reverse;

		Box::pin(
			futures_util::stream::once(async move {
				// Get the transaction sender based on snapshot mode
				let tx_sender = if snapshot {
					match self.ensure_snapshot_transaction().await {
						Ok(sender) => sender,
						Err(e) => return futures_util::stream::iter(vec![Err(e)]),
					}
				} else {
					match self.ensure_transaction().await {
						Ok(sender) => sender,
						Err(e) => return futures_util::stream::iter(vec![Err(e)]),
					}
				};

				let (response_tx, response_rx) = oneshot::channel();
				if let Err(_) = tx_sender
					.send(TransactionCommand::GetRange {
						begin_key: begin_selector.key().to_vec(),
						begin_or_equal: begin_selector.or_equal(),
						begin_offset: begin_selector.offset(),
						end_key: end_selector.key().to_vec(),
						end_or_equal: end_selector.or_equal(),
						end_offset: end_selector.offset(),
						limit,
						reverse,
						iteration: 1,
						response: response_tx,
					})
					.await
				{
					return futures_util::stream::iter(vec![Err(FdbError::from_code(1510))]);
				}

				match response_rx.await {
					Ok(Ok(result)) => {
						// Convert to FdbValues for the stream
						let values: Vec<_> = result
							.iter()
							.map(|kv| {
								Ok(crate::future::FdbValue::new(
									kv.key().to_vec(),
									kv.value().to_vec(),
								))
							})
							.collect();

						futures_util::stream::iter(values)
					}
					Ok(Err(e)) => futures_util::stream::iter(vec![Err(e)]),
					Err(_) => futures_util::stream::iter(vec![Err(FdbError::from_code(1510))]),
				}
			})
			.flatten(),
		)
	}

	fn set(&self, key: &[u8], value: &[u8]) {
		// Add write conflict range for this key
		let _ = self.conflict_tracker.add_range(
			self.tx_id,
			key,
			&[key, &[0u8]].concat(), // Key range is [key, key+\0)
			true,                    // is_write = true for writes
		);

		let mut state = self.state.lock().unwrap();
		state.operations.set(key, value);
	}

	fn clear(&self, key: &[u8]) {
		// Add write conflict range for this key
		let _ = self.conflict_tracker.add_range(
			self.tx_id,
			key,
			&[key, &[0u8]].concat(), // Key range is [key, key+\0)
			true,                    // is_write = true for writes
		);

		let mut state = self.state.lock().unwrap();
		state.operations.clear(key);
	}

	fn clear_range(&self, begin: &[u8], end: &[u8]) {
		// Add write conflict range for this range
		let _ = self.conflict_tracker.add_range(
			self.tx_id, begin, end, true, // is_write = true for writes
		);

		let mut state = self.state.lock().unwrap();
		state.operations.clear_range(begin, end);
	}

	fn commit(
		self: Box<Self>,
	) -> Pin<Box<dyn Future<Output = Result<TransactionCommitted, TransactionCommitError>> + Send>>
	{
		Box::pin(async move {
			// Get the operations and conflict ranges to commit
			let operations = {
				let mut state = self.state.lock().unwrap();
				if state.committed {
					return Err(TransactionCommitError::new(FdbError::from_code(2017)));
				}
				state.committed = true;

				state.operations.clone()
			};

			// Get the transaction sender
			let tx_sender = self
				.ensure_transaction()
				.await
				.map_err(|e| TransactionCommitError::new(e))?;

			// Send commit command with operations and conflict ranges
			let (response_tx, response_rx) = oneshot::channel();
			tx_sender
				.send(TransactionCommand::Commit {
					operations,
					response: response_tx,
				})
				.await
				.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?;

			// Wait for response
			let result = response_rx
				.await
				.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?;

			// Release conflict ranges after successful commit
			if result.is_ok() {
				self.conflict_tracker.release_transaction(self.tx_id);
			}

			result.map_err(|e| TransactionCommitError::new(e))
		})
	}

	fn reset(&mut self) {
		// Release any existing conflict ranges
		self.conflict_tracker.release_transaction(self.tx_id);

		// Generate a new transaction ID for the reset transaction
		self.tx_id = TransactionId::new();

		let mut state = self.state.lock().unwrap();
		*state = TransactionState::default();
		// Clear the transaction senders to reset connections
		self.tx_sender = Arc::new(OnceCell::new());
		self.snapshot_tx_sender = Arc::new(OnceCell::new());
	}

	fn cancel(&self) {
		// Release all conflict ranges for this transaction
		self.conflict_tracker.release_transaction(self.tx_id);

		// Send cancel command to both transaction tasks if they exist
		if let Some(tx_sender) = self.tx_sender.get() {
			let _ = tx_sender.try_send(TransactionCommand::Cancel);
		}
		if let Some(snapshot_tx_sender) = self.snapshot_tx_sender.get() {
			let _ = snapshot_tx_sender.try_send(TransactionCommand::Cancel);
		}
	}

	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> FdbResult<()> {
		// Determine if this is a write conflict range
		let is_write = match conflict_type {
			ConflictRangeType::Write => true,
			ConflictRangeType::Read => false,
		};

		// Add to the shared conflict tracker
		self.conflict_tracker
			.add_range(self.tx_id, begin, end, is_write)?;

		// Also store locally for later release
		let mut state = self.state.lock().unwrap();
		state
			.operations
			.add_conflict_range(begin, end, conflict_type);
		Ok(())
	}

	fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = FdbResult<i64>> + Send + 'a>> {
		let begin = begin.to_vec();
		let end = end.to_vec();

		Box::pin(async move {
			let tx_sender = self.ensure_snapshot_transaction().await?;

			// Send query command
			let (response_tx, response_rx) = oneshot::channel();
			tx_sender
				.send(TransactionCommand::GetEstimatedRangeSize {
					begin,
					end,
					response: response_tx,
				})
				.await
				.map_err(|_| FdbError::from_code(1510))?;

			// Wait for response
			let size = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

			Ok(size)
		})
	}

	fn commit_owned(&self) -> Pin<Box<dyn Future<Output = FdbResult<()>> + Send + '_>> {
		Box::pin(async move {
			// Get the operations to commit
			let operations = {
				let mut state = self.state.lock().unwrap();
				if state.committed {
					return Err(FdbError::from_code(2017));
				}
				state.committed = true;

				state.operations.clone()
			};

			// Get the transaction sender
			let tx_sender = self.ensure_transaction().await?;

			// Send commit command with operations
			let (response_tx, response_rx) = oneshot::channel();
			tx_sender
				.send(TransactionCommand::Commit {
					operations,
					response: response_tx,
				})
				.await
				.map_err(|_| FdbError::from_code(1510))?;

			// Wait for response
			let result = response_rx.await.map_err(|_| FdbError::from_code(1510))?;

			// Release conflict ranges after successful commit
			if result.is_ok() {
				self.conflict_tracker.release_transaction(self.tx_id);
			}

			result.map(|_| ())
		})
	}
}
