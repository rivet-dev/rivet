use std::{
	path::PathBuf,
	sync::{Arc, Mutex},
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

use super::{conflict_range_tracker::ConflictRangeTracker, transaction::RocksDbTransactionDriver};

pub struct RocksDbDatabaseDriver {
	db: Arc<OptimisticTransactionDB>,
	max_retries: Arc<Mutex<i32>>,
	conflict_tracker: ConflictRangeTracker,
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
		opts.set_max_total_wal_size(64 * 1024 * 1024); // 64MB

		// Open the OptimisticTransactionDB
		tracing::debug!(path=%db_path.display(), "opening rocksdb");
		let db = OptimisticTransactionDB::open(&opts, db_path).context("failed to open rocksdb")?;

		Ok(RocksDbDatabaseDriver {
			db: Arc::new(db),
			max_retries: Arc::new(Mutex::new(100)),
			conflict_tracker: ConflictRangeTracker::new(),
		})
	}
}

impl DatabaseDriver for RocksDbDatabaseDriver {
	fn create_trx(&self) -> Result<Transaction> {
		Ok(Transaction::new(Arc::new(RocksDbTransactionDriver::new(
			self.db.clone(),
			self.conflict_tracker.clone(),
		))))
	}

	fn run<'a>(
		&'a self,
		closure: Box<dyn Fn(RetryableTransaction) -> BoxFut<'a, Result<Erased>> + Send + Sync + 'a>,
	) -> BoxFut<'a, Result<Erased>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = *self.max_retries.lock().unwrap();

			for attempt in 0..max_retries {
				let tx = self.create_trx()?;
				let mut retryable = RetryableTransaction::new(tx);
				retryable.maybe_committed = maybe_committed;

				// Execute transaction
				let error = match closure(retryable.clone()).await {
					Ok(res) => match retryable.inner.driver.commit_ref().await {
						Ok(_) => return Ok(res),
						Err(e) => e,
					},
					Err(e) => e,
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
				*self.max_retries.lock().unwrap() = limit;
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
