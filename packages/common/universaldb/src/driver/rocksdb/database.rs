use std::{
	path::PathBuf,
	sync::{Arc, Mutex},
};

use rocksdb::{OptimisticTransactionDB, Options};

use crate::{
	FdbBindingError, FdbError, FdbResult, MaybeCommitted, RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased},
	options::DatabaseOption,
	utils::calculate_tx_retry_backoff,
};

use super::{conflict_range_tracker::ConflictRangeTracker, transaction::RocksDbTransactionDriver};

pub struct RocksDbDatabaseDriver {
	db: Arc<OptimisticTransactionDB>,
	max_retries: Arc<Mutex<i32>>,
	conflict_tracker: ConflictRangeTracker,
}

impl RocksDbDatabaseDriver {
	pub async fn new(db_path: PathBuf) -> FdbResult<Self> {
		// Create directory if it doesn't exist
		std::fs::create_dir_all(&db_path).map_err(|_| FdbError::from_code(1510))?;

		// Configure RocksDB options
		let mut opts = Options::default();
		opts.create_if_missing(true);
		opts.set_max_open_files(10000);
		opts.set_keep_log_file_num(10);
		opts.set_max_total_wal_size(64 * 1024 * 1024); // 64MB

		// Open the OptimisticTransactionDB
		let db =
			OptimisticTransactionDB::open(&opts, db_path).map_err(|_| FdbError::from_code(1510))?;

		Ok(RocksDbDatabaseDriver {
			db: Arc::new(db),
			max_retries: Arc::new(Mutex::new(100)),
			conflict_tracker: ConflictRangeTracker::new(),
		})
	}
}

impl DatabaseDriver for RocksDbDatabaseDriver {
	fn create_trx(&self) -> FdbResult<Transaction> {
		Ok(Transaction::new(Box::new(RocksDbTransactionDriver::new(
			self.db.clone(),
			self.conflict_tracker.clone(),
		))))
	}

	fn run<'a>(
		&'a self,
		closure: Box<
			dyn Fn(
					RetryableTransaction,
					MaybeCommitted,
				) -> BoxFut<'a, Result<Erased, FdbBindingError>>
				+ Send
				+ Sync
				+ 'a,
		>,
	) -> BoxFut<'a, Result<Erased, FdbBindingError>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = *self.max_retries.lock().unwrap();

			for attempt in 0..max_retries {
				let tx = self.create_trx()?;
				let retryable = RetryableTransaction::new(tx);

				// Execute transaction
				let result = closure(retryable.clone(), maybe_committed).await;
				let fdb_error = match result {
					std::result::Result::Ok(res) => {
						match retryable.inner.driver.commit_owned().await {
							Ok(_) => return Ok(res),
							Err(e) => e,
						}
					}
					std::result::Result::Err(e) => {
						if let Some(fdb_error) = e.get_fdb_error() {
							fdb_error
						} else {
							return Err(e);
						}
					}
				};

				// Handle retry or return error
				if fdb_error.is_retryable() {
					if fdb_error.is_maybe_committed() {
						maybe_committed = MaybeCommitted(true);
					}

					let backoff_ms = calculate_tx_retry_backoff(attempt as usize);
					tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
				} else {
					return Err(FdbBindingError::from(fdb_error));
				}
			}

			// Max retries exceeded
			Err(FdbBindingError::from(FdbError::from_code(1007)))
		})
	}

	fn set_option(&self, opt: DatabaseOption) -> FdbResult<()> {
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
