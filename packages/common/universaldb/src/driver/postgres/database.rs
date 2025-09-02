use std::sync::{Arc, Mutex};

use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

use crate::{
	FdbBindingError, FdbError, FdbResult, MaybeCommitted, RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased},
	options::DatabaseOption,
	utils::calculate_tx_retry_backoff,
};

use super::transaction::PostgresTransactionDriver;

pub struct PostgresDatabaseDriver {
	pool: Arc<Pool>,
	max_retries: Arc<Mutex<i32>>,
}

impl PostgresDatabaseDriver {
	pub async fn new(connection_string: String) -> FdbResult<Self> {
		tracing::debug!(connection_string = ?connection_string, "Creating PostgresDatabaseDriver");

		// Create deadpool config from connection string
		let mut config = Config::new();
		config.url = Some(connection_string);
		config.pool = Some(PoolConfig {
			max_size: 64,
			..Default::default()
		});
		config.manager = Some(ManagerConfig {
			recycling_method: RecyclingMethod::Fast,
		});

		tracing::debug!("Creating Postgres pool");
		// Create the pool
		let pool = config
			.create_pool(Some(Runtime::Tokio1), NoTls)
			.map_err(|e| {
				tracing::error!(error = ?e, "Failed to create Postgres pool");
				FdbError::from_code(1510)
			})?;

		tracing::debug!("Getting Postgres connection from pool");
		// Get a connection from the pool to create the table
		let conn = pool.get().await.map_err(|e| {
			tracing::error!(error = ?e, "Failed to get Postgres connection");
			FdbError::from_code(1510)
		})?;

		// Enable btree gist
		conn.execute("CREATE EXTENSION IF NOT EXISTS btree_gist", &[])
			.await
			.map_err(|_| FdbError::from_code(1510))?;

		// Create the KV table if it doesn't exist
		conn.execute(
			"CREATE TABLE IF NOT EXISTS kv (
				key BYTEA PRIMARY KEY,
				value BYTEA NOT NULL
			)",
			&[],
		)
		.await
		.map_err(|_| FdbError::from_code(1510))?;

		// Create range_type type if it doesn't exist
		conn.execute(
			"DO $$ BEGIN
				CREATE TYPE range_type AS ENUM ('read', 'write');
			EXCEPTION
				WHEN duplicate_object THEN null;
			END $$",
			&[],
		)
		.await
		.map_err(|_| FdbError::from_code(1510))?;

		// Create bytearange type if it doesn't exist
		conn.execute(
			"DO $$ BEGIN
				CREATE TYPE bytearange AS RANGE (
					SUBTYPE = bytea,
					SUBTYPE_OPCLASS = bytea_ops
				);
			EXCEPTION
				WHEN duplicate_object THEN null;
			END $$",
			&[],
		)
		.await
		.map_err(|_| FdbError::from_code(1510))?;

		// Create the conflict ranges table for non-snapshot reads
		// This enforces consistent reads for ranges by preventing overlapping conflict ranges
		conn.execute(
			"CREATE UNLOGGED TABLE IF NOT EXISTS conflict_ranges (
				range_data BYTEARANGE NOT NULL,
				conflict_type range_type NOT NULL,
				txn_id BIGINT NOT NULL DEFAULT txid_current(),
				
				-- This constraint prevents read-write conflicts ONLY between different transactions
				-- Same transaction can have any combination of read/write overlaps
				EXCLUDE USING gist (
					range_data WITH &&, 
					conflict_type WITH <>,
					txn_id WITH <>
				)
			)",
			&[],
		)
		.await
		.map_err(|_| FdbError::from_code(1510))?;

		// Connection is automatically returned to the pool when dropped
		drop(conn);

		Ok(PostgresDatabaseDriver {
			pool: Arc::new(pool),
			max_retries: Arc::new(Mutex::new(100)),
		})
	}
}

impl DatabaseDriver for PostgresDatabaseDriver {
	fn create_trx(&self) -> FdbResult<Transaction> {
		// Pass the connection pool to the transaction driver
		Ok(Transaction::new(Box::new(PostgresTransactionDriver::new(
			self.pool.clone(),
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

			Err(FdbBindingError::from(FdbError::from_code(1007))) // Retry limit exceeded
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
