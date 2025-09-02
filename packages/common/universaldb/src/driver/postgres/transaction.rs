use std::{
	future::Future,
	pin::Pin,
	sync::{Arc, Mutex},
};

use deadpool_postgres::Pool;
use tokio::sync::{OnceCell, mpsc, oneshot};

use crate::{
	FdbError, FdbResult, KeySelector, RangeOption, TransactionCommitError, TransactionCommitted,
	driver::TransactionDriver,
	future::{FdbSlice, FdbValues},
	options::{ConflictRangeType, MutationType},
	tx_ops::{Operation, TransactionOperations},
};

use super::transaction_task::{TransactionCommand, TransactionIsolationLevel, TransactionTask};

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

pub struct PostgresTransactionDriver {
	pool: Arc<Pool>,
	state: Arc<Mutex<TransactionState>>,
	tx_sender: Arc<OnceCell<mpsc::Sender<TransactionCommand>>>,
	snapshot_tx_sender: Arc<OnceCell<mpsc::Sender<TransactionCommand>>>,
}

impl PostgresTransactionDriver {
	pub fn new(pool: Arc<Pool>) -> Self {
		PostgresTransactionDriver {
			pool,
			state: Arc::new(Mutex::new(TransactionState::default())),
			tx_sender: Arc::new(OnceCell::new()),
			snapshot_tx_sender: Arc::new(OnceCell::new()),
		}
	}

	/// Get or create the transaction task
	async fn ensure_transaction(&self) -> FdbResult<&mpsc::Sender<TransactionCommand>> {
		self.tx_sender
			.get_or_try_init(|| async {
				let (sender, receiver) = mpsc::channel(100);

				// Spawn the transaction task with serializable isolation
				let task = TransactionTask::new(
					self.pool.as_ref().clone(),
					receiver,
					TransactionIsolationLevel::Serializable,
				);
				tokio::spawn(task.run());

				Ok(sender)
			})
			.await
			.map_err(|_: anyhow::Error| FdbError::from_code(1510))
	}

	/// Get or create the snapshot transaction task
	/// This creates a separate REPEATABLE READ READ ONLY transaction
	/// to enforce reading from a consistent snapshot
	async fn ensure_snapshot_transaction(&self) -> FdbResult<&mpsc::Sender<TransactionCommand>> {
		self.snapshot_tx_sender
			.get_or_try_init(|| async {
				let (sender, receiver) = mpsc::channel(100);

				// Spawn the transaction task with repeatable read read-only isolation
				let task = TransactionTask::new(
					self.pool.as_ref().clone(),
					receiver,
					TransactionIsolationLevel::RepeatableReadReadOnly,
				);
				tokio::spawn(task.run());

				Ok(sender)
			})
			.await
			.map_err(|_: anyhow::Error| FdbError::from_code(1510))
	}
}

impl TransactionDriver for PostgresTransactionDriver {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.atomic_op(key, param, op_type);
		}
	}

	fn get<'a>(
		&'a self,
		key: &[u8],
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<Option<FdbSlice>>> + Send + 'a>> {
		let key = key.to_vec();
		Box::pin(async move {
			// Both snapshot and non-snapshot reads check local operations first
			// This matches FoundationDB behavior where transactions see their own writes
			let ops = {
				let state = self.state.lock().unwrap();
				state.operations.clone()
			};

			ops.get_with_callback(&key, || async {
				let tx_sender = if snapshot {
					self.ensure_snapshot_transaction().await?
				} else {
					self.ensure_transaction().await?
				};

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
			// This matches FoundationDB behavior where transactions see their own writes
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
		_iteration: usize,
		snapshot: bool,
	) -> Pin<Box<dyn Future<Output = FdbResult<FdbValues>> + Send + 'a>> {
		let opt = opt.clone();

		Box::pin(async move {
			let begin = opt.begin.key().to_vec();
			let begin_or_equal = opt.begin.or_equal();
			let begin_offset = opt.begin.offset();
			let end = opt.end.key().to_vec();
			let end_or_equal = opt.end.or_equal();
			let end_offset = opt.end.offset();
			let limit = opt.limit;
			let reverse = opt.reverse;

			// Both snapshot and non-snapshot reads check local operations first
			// This matches FoundationDB behavior where transactions see their own writes
			let ops = {
				let state = self.state.lock().unwrap();
				state.operations.clone()
			};

			ops.get_range(&opt, || async {
				let tx_sender = if snapshot {
					self.ensure_snapshot_transaction().await?
				} else {
					self.ensure_transaction().await?
				};

				// Send query command
				let (response_tx, response_rx) = oneshot::channel();
				tx_sender
					.send(TransactionCommand::GetRange {
						begin: begin.clone(),
						begin_or_equal,
						begin_offset,
						end: end.clone(),
						end_or_equal,
						end_offset,
						limit,
						reverse,
						response: response_tx,
					})
					.await
					.map_err(|_| FdbError::from_code(1510))?;

				// Wait for response
				let keyvalues_data = response_rx.await.map_err(|_| FdbError::from_code(1510))??;

				let keyvalues: Vec<_> = keyvalues_data
					.into_iter()
					.map(|(key, value)| crate::future::FdbKeyValue::new(key, value))
					.collect();

				Ok(crate::future::FdbValues::new(keyvalues))
			})
			.await
		})
	}

	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		snapshot: bool,
	) -> crate::future::FdbStream<'a, crate::future::FdbValue> {
		use futures_util::{StreamExt, stream};

		// Convert the range result into a stream
		let fut = async move {
			match self.get_range(&opt, 1, snapshot).await {
				Ok(values) => values
					.into_iter()
					.map(|kv| Ok(crate::future::FdbValue::from_keyvalue(kv)))
					.collect::<Vec<_>>(),
				Err(e) => vec![Err(e)],
			}
		};

		Box::pin(stream::once(fut).flat_map(stream::iter))
	}

	fn set(&self, key: &[u8], value: &[u8]) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.set(key, value);
		}
	}

	fn clear(&self, key: &[u8]) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.clear(key);
		}
	}

	fn clear_range(&self, begin: &[u8], end: &[u8]) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.clear_range(begin, end);
		}
	}

	fn commit(
		self: Box<Self>,
	) -> Pin<Box<dyn Future<Output = Result<TransactionCommitted, TransactionCommitError>> + Send>>
	{
		Box::pin(async move {
			// Get operations and mark as committed
			let operations = {
				let mut state = self.state.lock().unwrap();
				if state.committed {
					return Ok(());
				}
				state.committed = true;

				state.operations.clone()
			};

			// Get the transaction sender if it exists
			let tx_sender = self.tx_sender.get();

			// If we have a transaction task, execute operations and commit
			if let Some(sender) = tx_sender {
				// Execute all buffered operations
				for op in operations.operations() {
					match op {
						Operation::Set { key, value } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::Set {
									key: key.clone(),
									value: value.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::Clear { key } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::Clear {
									key: key.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::ClearRange { begin, end } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::ClearRange {
									begin: begin.clone(),
									end: end.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::AtomicOp {
							key,
							param,
							op_type,
						} => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::AtomicOp {
									key: key.clone(),
									param: param.clone(),
									op_type: *op_type,
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
					}
				}

				// Send commit command
				let (response_tx, response_rx) = oneshot::channel();
				sender
					.send(TransactionCommand::Commit {
						has_conflict_ranges: !operations.conflict_ranges().is_empty(),
						response: response_tx,
					})
					.await
					.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?;

				// Wait for commit response
				response_rx
					.await
					.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?
					.map_err(TransactionCommitError::new)?;
			} else if !operations.operations().is_empty() {
				// We have operations but no transaction - create one just for commit
				let tx_sender = self
					.ensure_transaction()
					.await
					.map_err(TransactionCommitError::new)?;

				// Execute all operations
				for op in operations.operations() {
					match op {
						Operation::Set { key, value } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::Set {
									key: key.clone(),
									value: value.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::Clear { key } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::Clear {
									key: key.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::ClearRange { begin, end } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::ClearRange {
									begin: begin.clone(),
									end: end.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
						Operation::AtomicOp {
							key,
							param,
							op_type,
						} => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::AtomicOp {
									key: key.clone(),
									param: param.clone(),
									op_type: *op_type,
									response: response_tx,
								})
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?;

							response_rx
								.await
								.map_err(|_| {
									TransactionCommitError::new(FdbError::from_code(1510))
								})?
								.map_err(TransactionCommitError::new)?;
						}
					}
				}

				// Send commit command
				let (response_tx, response_rx) = oneshot::channel();
				tx_sender
					.send(TransactionCommand::Commit {
						has_conflict_ranges: !operations.conflict_ranges().is_empty(),
						response: response_tx,
					})
					.await
					.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?;

				// Wait for commit response
				response_rx
					.await
					.map_err(|_| TransactionCommitError::new(FdbError::from_code(1510)))?
					.map_err(TransactionCommitError::new)?;
			}

			Ok(())
		})
	}

	fn reset(&mut self) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.clear_all();
			state.committed = false;
		}
		// Note: We can't reset the transaction once it's created
		// The transaction task will continue running
	}

	fn cancel(&self) {
		if let Ok(mut state) = self.state.lock() {
			state.operations.clear_all();
			state.committed = true; // Prevent future commits
		}
		// Transaction will be rolled back when dropped
	}

	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> FdbResult<()> {
		// For PostgreSQL, we implement conflict ranges using the conflict_ranges table
		// This ensures serializable isolation for the specified range

		// Track this conflict range in TransactionOperations
		{
			let mut state = self.state.lock().unwrap();
			state
				.operations
				.add_conflict_range(begin, end, conflict_type);
		}

		// Get the transaction sender
		let tx_sender = match self.tx_sender.get() {
			Some(sender) => sender,
			None => {
				// If no transaction exists yet, we need to create one
				// This is a synchronous method, so we can't use async here
				// For now, we'll just return Ok as the conflict range will be added
				// when the transaction actually performs operations
				return Ok(());
			}
		};

		// Clone keys for the conflict range command
		let begin_vec = begin.to_vec();
		let end_vec = end.to_vec();

		// Try to send the add conflict range command
		// Since this is a synchronous method, we use try_send
		let (response_tx, _response_rx) = oneshot::channel();
		match tx_sender.try_send(TransactionCommand::AddConflictRange {
			begin: begin_vec,
			end: end_vec,
			conflict_type,
			response: response_tx,
		}) {
			Ok(_) => {
				// Command sent successfully
				// Note: We can't wait for the response in a sync method
				// The actual conflict range acquisition will happen asynchronously
				Ok(())
			}
			Err(_) => {
				// Channel is full or closed
				// Return an error indicating we couldn't add the conflict range
				Err(FdbError::from_code(1020)) // Transaction conflict error
			}
		}
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
			// Get operations and mark as committed
			let operations = {
				let mut state = self.state.lock().unwrap();
				if state.committed {
					return Ok(());
				}
				state.committed = true;

				state.operations.clone()
			};

			// Get the transaction sender if it exists
			let tx_sender = self.tx_sender.get();

			// If we have a transaction task, execute operations and commit
			if let Some(sender) = tx_sender {
				// Execute all buffered operations
				for op in operations.operations() {
					match op {
						Operation::Set { key, value } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::Set {
									key: key.clone(),
									value: value.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::Clear { key } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::Clear {
									key: key.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::ClearRange { begin, end } => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::ClearRange {
									begin: begin.clone(),
									end: end.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::AtomicOp {
							key,
							param,
							op_type,
						} => {
							let (response_tx, response_rx) = oneshot::channel();
							sender
								.send(TransactionCommand::AtomicOp {
									key: key.clone(),
									param: param.clone(),
									op_type: *op_type,
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
					}
				}

				// Send commit command
				let (response_tx, response_rx) = oneshot::channel();
				sender
					.send(TransactionCommand::Commit {
						has_conflict_ranges: !operations.conflict_ranges().is_empty(),
						response: response_tx,
					})
					.await
					.map_err(|_| FdbError::from_code(1510))?;

				// Wait for commit response
				response_rx.await.map_err(|_| FdbError::from_code(1510))??;
			} else if !operations.operations().is_empty() {
				// We have operations but no transaction - create one just for commit
				let tx_sender = self.ensure_transaction().await?;

				// Execute all operations
				for op in operations.operations() {
					match op {
						Operation::Set { key, value } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::Set {
									key: key.clone(),
									value: value.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::Clear { key } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::Clear {
									key: key.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::ClearRange { begin, end } => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::ClearRange {
									begin: begin.clone(),
									end: end.clone(),
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
						Operation::AtomicOp {
							key,
							param,
							op_type,
						} => {
							let (response_tx, response_rx) = oneshot::channel();
							tx_sender
								.send(TransactionCommand::AtomicOp {
									key: key.clone(),
									param: param.clone(),
									op_type: *op_type,
									response: response_tx,
								})
								.await
								.map_err(|_| FdbError::from_code(1510))?;

							response_rx.await.map_err(|_| FdbError::from_code(1510))??;
						}
					}
				}

				// Send commit command
				let (response_tx, response_rx) = oneshot::channel();
				tx_sender
					.send(TransactionCommand::Commit {
						has_conflict_ranges: !operations.conflict_ranges().is_empty(),
						response: response_tx,
					})
					.await
					.map_err(|_| FdbError::from_code(1510))?;

				// Wait for commit response
				response_rx.await.map_err(|_| FdbError::from_code(1510))??;
			}

			Ok(())
		})
	}
}
