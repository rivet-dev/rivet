use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::{Context, Result};
use rocksdb::OptimisticTransactionDB;
use tokio::sync::{OnceCell, mpsc, oneshot};

use crate::{
	driver::TransactionDriver,
	key_selector::KeySelector,
	options::{ConflictRangeType, MutationType},
	range_option::RangeOption,
	tx_ops::TransactionOperations,
	utils::{IsolationLevel, end_of_key_range},
	value::{Slice, Value, Values},
};

use super::{
	conflict_range_tracker::{ConflictRangeTracker, TransactionId},
	transaction_task::{TransactionCommand, TransactionTask},
};

pub struct RocksDbTransactionDriver {
	db: Arc<OptimisticTransactionDB>,
	operations: TransactionOperations,
	committed: AtomicBool,
	tx_sender: OnceCell<mpsc::Sender<TransactionCommand>>,
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
			operations: TransactionOperations::default(),
			committed: AtomicBool::new(false),
			tx_sender: OnceCell::new(),
			conflict_tracker,
			tx_id: TransactionId::new(),
		}
	}

	/// Get or create the transaction task for non-snapshot operations
	async fn ensure_transaction(&self) -> Result<&mpsc::Sender<TransactionCommand>> {
		self.tx_sender
			.get_or_try_init(|| async {
				let (sender, receiver) = mpsc::channel(100);

				// Spawn the transaction task
				let task = TransactionTask::new(self.db.clone(), receiver);
				tokio::spawn(task.run());

				anyhow::Ok(sender)
			})
			.await
			.context("failed to initialize transaction task")
	}
}

impl TransactionDriver for RocksDbTransactionDriver {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.operations.atomic_op(key, param, op_type);
	}

	fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Option<Slice>>> + Send + 'a>> {
		let key = key.to_vec();
		Box::pin(async move {
			self.operations
				.get_with_callback(&key, isolation_level, || async {
					if let IsolationLevel::Serializable = isolation_level {
						self.conflict_tracker.add_range(
							self.tx_id,
							&key,
							&end_of_key_range(&key),
							false, // is_write = false for reads
						)?;
					}

					let tx_sender = self.ensure_transaction().await?;

					// Send query command
					let (response_tx, response_rx) = oneshot::channel();
					tx_sender
						.send(TransactionCommand::Get {
							key: key.to_vec(),
							response: response_tx,
						})
						.await
						.context("failed to send transaction command")?;

					// Wait for response
					let value = response_rx
						.await
						.context("failed to receive transaction response")??;

					Ok(value)
				})
				.await
		})
	}

	fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Slice>> + Send + 'a>> {
		let selector = selector.clone();

		Box::pin(async move {
			let key = selector.key().to_vec();
			let offset = selector.offset();
			let or_equal = selector.or_equal();

			self.operations
				.get_key(&selector, isolation_level, || async {
					if let IsolationLevel::Serializable = isolation_level {
						self.conflict_tracker.add_range(
							self.tx_id,
							&key,
							&end_of_key_range(&key),
							false, // is_write = false for reads
						)?;
					}

					let tx_sender = self.ensure_transaction().await?;

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
						.context("failed to send commit command")?;

					// Wait for response
					let result_key = response_rx
						.await
						.context("failed to receive key selector response")??;

					// Return the key if found, or empty vector if not
					Ok(result_key.unwrap_or_else(Slice::new))
				})
				.await
		})
	}

	fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Values>> + Send + 'a>> {
		// Extract fields from RangeOption for the async closure
		let opt = opt.clone();
		let begin_selector = opt.begin.clone();
		let end_selector = opt.end.clone();
		let limit = opt.limit;
		let reverse = opt.reverse;

		Box::pin(async move {
			self.operations
				.get_range(&opt, isolation_level, || async {
					if let IsolationLevel::Serializable = isolation_level {
						// Add read conflict range for this range (using raw keys, conservative)
						self.conflict_tracker.add_range(
							self.tx_id,
							begin_selector.key(),
							end_selector.key(),
							false, // is_write = false for reads
						)?;
					}

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
						.context("failed to send transaction command")?;

					// Wait for response
					let values = response_rx
						.await
						.context("failed to receive range response")??;

					Ok(values)
				})
				.await
		})
	}

	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		use futures_util::{StreamExt, stream};

		// Convert the range result into a stream
		let fut = async move {
			match self.get_range(&opt, 1, isolation_level).await {
				Ok(values) => values
					.into_iter()
					.map(|kv| Ok(Value::from_keyvalue(kv)))
					.collect::<Vec<_>>(),
				Err(e) => vec![Err(e)],
			}
		};

		Box::pin(stream::once(fut).flat_map(stream::iter))
	}

	fn set(&self, key: &[u8], value: &[u8]) {
		// Add write conflict range for this range
		let _ = self.conflict_tracker.add_range(
			self.tx_id,
			key,
			&end_of_key_range(&key),
			true, // is_write = true for writes
		);

		self.operations.set(key, value);
	}

	fn clear(&self, key: &[u8]) {
		// Add write conflict range for this range
		let _ = self.conflict_tracker.add_range(
			self.tx_id,
			key,
			&end_of_key_range(&key),
			true, // is_write = true for writes
		);

		self.operations.clear(key);
	}

	fn clear_range(&self, begin: &[u8], end: &[u8]) {
		// Add write conflict range for this range
		let _ = self.conflict_tracker.add_range(
			self.tx_id, begin, end, true, // is_write = true for writes
		);

		self.operations.clear_range(begin, end);
	}

	fn commit(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		Box::pin(async move {
			if self.committed.load(Ordering::SeqCst) {
				return Ok(());
			}
			self.committed.store(true, Ordering::SeqCst);

			let (operations, _conflict_ranges) = self.operations.consume();

			// Get the transaction sender
			let tx_sender = self.ensure_transaction().await?;

			// Send commit command with operations and conflict ranges
			let (response_tx, response_rx) = oneshot::channel();
			tx_sender
				.send(TransactionCommand::Commit {
					operations,
					response: response_tx,
				})
				.await
				.context("failed to send commit command")?;

			// Wait for response
			let result = response_rx
				.await
				.context("failed to receive commit response")?;

			// Release conflict ranges after successful commit
			if result.is_ok() {
				self.conflict_tracker.release_transaction(self.tx_id);
			}

			result
		})
	}

	fn reset(&mut self) {
		// Release any existing conflict ranges
		self.conflict_tracker.release_transaction(self.tx_id);

		// Generate a new transaction ID for the reset transaction
		self.tx_id = TransactionId::new();

		self.operations.clear_all();
		// Clear the transaction senders to reset connections
		self.tx_sender = OnceCell::new();
	}

	fn cancel(&self) {
		// Release all conflict ranges for this transaction
		self.conflict_tracker.release_transaction(self.tx_id);

		// Send cancel command to both transaction tasks if they exist
		if let Some(tx_sender) = self.tx_sender.get() {
			let _ = tx_sender.try_send(TransactionCommand::Cancel);
		}
	}

	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		let is_write = match conflict_type {
			ConflictRangeType::Write => true,
			ConflictRangeType::Read => false,
		};

		self.conflict_tracker
			.add_range(self.tx_id, begin, end, is_write)?;

		self.operations
			.add_conflict_range(begin, end, conflict_type);

		Ok(())
	}

	fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		let begin = begin.to_vec();
		let end = end.to_vec();

		Box::pin(async move {
			let tx_sender = self.ensure_transaction().await?;

			// Send query command
			let (response_tx, response_rx) = oneshot::channel();
			tx_sender
				.send(TransactionCommand::GetEstimatedRangeSize {
					begin,
					end,
					response: response_tx,
				})
				.await
				.context("failed to send commit command")?;

			// Wait for response
			let size = response_rx
				.await
				.context("failed to receive size response")??;

			Ok(size)
		})
	}

	fn commit_ref(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async move {
			if self.committed.load(Ordering::SeqCst) {
				return Ok(());
			}
			self.committed.store(true, Ordering::SeqCst);

			let (operations, _conflict_ranges) = self.operations.consume();

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
				.context("failed to send commit command")?;

			// Wait for response
			let result = response_rx
				.await
				.context("failed to receive commit response")?;

			// Release conflict ranges after successful commit
			if result.is_ok() {
				self.conflict_tracker.release_transaction(self.tx_id);
			}

			result.map(|_| ())
		})
	}
}
