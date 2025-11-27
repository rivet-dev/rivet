use std::{
	path::PathBuf,
	sync::{
		Arc,
		atomic::{AtomicI32, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result};
use rocksdb::{OptimisticTransactionDB, Options};

use crate::{
	RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased},
	error::DatabaseError,
	options::DatabaseOption,
	utils::{MaybeCommitted, calculate_tx_retry_backoff},
};

use super::{
	transaction::RocksDbTransactionDriver, transaction_conflict_tracker::TransactionConflictTracker,
};

const TXN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct RocksDbDatabaseDriver {
	db: Arc<OptimisticTransactionDB>,
	max_retries: AtomicI32,
	txn_conflict_tracker: TransactionConflictTracker,
}

impl RocksDbDatabaseDriver {
	pub async fn new(db_path: PathBuf) -> Result<Self> {
		tracing::info!(?db_path, "starting file system driver");

		// Create directory if it doesn't exist
		std::fs::create_dir_all(&db_path).context("failed to create database directory")?;

		// Configure RocksDB options
		let mut opts = Options::default();
		opts.create_if_missing(true);
		opts.set_max_open_files(10000);
		opts.set_keep_log_file_num(10);
		opts.set_max_total_wal_size(64 * 1024 * 1024); // 64MiB
		opts.set_write_buffer_size(256 * 1024 * 1024); // 256MiB for conflict detection

		// Open the OptimisticTransactionDB
		tracing::debug!(path=%db_path.display(), "opening rocksdb");
		let db = OptimisticTransactionDB::open(&opts, db_path).context("failed to open rocksdb")?;

		Ok(RocksDbDatabaseDriver {
			db: Arc::new(db),
			max_retries: AtomicI32::new(100),
			txn_conflict_tracker: TransactionConflictTracker::new(),
		})
	}
}

impl DatabaseDriver for RocksDbDatabaseDriver {
	fn create_trx(&self) -> Result<Transaction> {
		Ok(Transaction::new(Arc::new(RocksDbTransactionDriver::new(
			self.db.clone(),
			self.txn_conflict_tracker.clone(),
		))))
	}

	fn run<'a>(
		&'a self,
		closure: Box<dyn Fn(RetryableTransaction) -> BoxFut<'a, Result<Erased>> + Send + Sync + 'a>,
	) -> BoxFut<'a, Result<Erased>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = self.max_retries.load(Ordering::SeqCst);

			for attempt in 0..max_retries {
				let tx = self.create_trx()?;
				let mut retryable = RetryableTransaction::new(tx);
				retryable.maybe_committed = maybe_committed;

				// Execute transaction
				let error =
					match tokio::time::timeout(TXN_TIMEOUT, closure(retryable.clone())).await {
						Ok(Ok(res)) => match retryable.inner.driver.commit_ref().await {
							Ok(_) => return Ok(res),
							Err(e) => e,
						},
						Ok(Err(e)) => e,
						Err(_) => anyhow::Error::from(DatabaseError::TransactionTooOld),
					};

				let chain = error
					.chain()
					.find_map(|x| x.downcast_ref::<DatabaseError>());

				if let Some(db_error) = chain {
					// Handle retry or return error
					if db_error.is_retryable() {
						if db_error.is_maybe_committed() {
							maybe_committed = MaybeCommitted(true);
						}

						let backoff_ms = calculate_tx_retry_backoff(attempt as usize);
						tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
						continue;
					}
				}

				return Err(error);
			}

			Err(DatabaseError::MaxRetriesReached.into())
		})
	}

	fn set_option(&self, opt: DatabaseOption) -> Result<()> {
		match opt {
			DatabaseOption::TransactionRetryLimit(limit) => {
				self.max_retries.store(limit, Ordering::SeqCst);
				Ok(())
			}
		}
	}
}

impl Drop for RocksDbDatabaseDriver {
	fn drop(&mut self) {
		self.db.cancel_all_background_work(true);
	}
}
