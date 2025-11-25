use std::{
	sync::{
		Arc,
		atomic::{AtomicI32, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result};
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use tokio::task::JoinHandle;
use tokio_postgres::NoTls;

use crate::{
	RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased},
	error::DatabaseError,
	options::DatabaseOption,
	utils::{MaybeCommitted, calculate_tx_retry_backoff},
};

use super::transaction::PostgresTransactionDriver;

const TXN_TIMEOUT: Duration = Duration::from_secs(5);
const GC_INTERVAL: Duration = Duration::from_secs(5);

pub struct PostgresDatabaseDriver {
	pool: Arc<Pool>,
	max_retries: AtomicI32,
	gc_handle: JoinHandle<()>,
}

impl PostgresDatabaseDriver {
	pub async fn new(connection_string: String) -> Result<Self> {
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
			.context("failed to create postgres connection pool")?;

		tracing::debug!("Getting Postgres connection from pool");
		// Get a connection from the pool to create the table
		let conn = pool
			.get()
			.await
			.context("failed to get connection from postgres pool")?;

		// Enable btree gist
		conn.execute("CREATE EXTENSION IF NOT EXISTS btree_gist", &[])
			.await
			.context("failed to create btree_gist extension")?;

		conn.execute("CREATE UNLOGGED SEQUENCE IF NOT EXISTS global_version_seq START WITH 1 INCREMENT BY 1 MINVALUE 1", &[])
			.await
			.context("failed to create global version sequence")?;

		// Create the KV table if it doesn't exist
		conn.execute(
			"CREATE TABLE IF NOT EXISTS kv (
				key BYTEA PRIMARY KEY,
				value BYTEA NOT NULL
			)",
			&[],
		)
		.await
		.context("failed to create kv table")?;

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
		.context("failed to create range_type enum")?;

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
		.context("failed to create bytearange type")?;

		// Create the conflict ranges table for non-snapshot reads
		// This enforces consistent reads for ranges by preventing overlapping conflict ranges
		conn.execute(
			"CREATE UNLOGGED TABLE IF NOT EXISTS conflict_ranges (
				range_data BYTEARANGE NOT NULL,
				conflict_type range_type NOT NULL,
				start_version BIGINT NOT NULL,
				commit_version BIGINT NOT NULL,
				ts timestamp NOT NULL DEFAULT now(),

				EXCLUDE USING gist (
					-- Conflict if byte range overlaps...
					range_data WITH &&,
					-- And f conflict types are different...
					conflict_type WITH <>,
					-- And f the txn versions overlap...
					int8range(start_version, commit_version, '[]') WITH &&,
					-- But not if the start_version is the same (from the same txn)
					start_version WITH <>
				)
			)",
			&[],
		)
		.await
		.context("failed to create conflict_ranges table")?;

		// Create index on ts column for efficient garbage collection
		conn.execute(
			"CREATE INDEX IF NOT EXISTS idx_conflict_ranges_ts ON conflict_ranges (ts)",
			&[],
		)
		.await
		.context("failed to create index on conflict_ranges ts column")?;

		let gc_handle = tokio::spawn(async move {
			let mut interval = tokio::time::interval(GC_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;

				// NOTE: Transactions have a max limit of 5 seconds, we delete after 10 seconds for extra padding
				// Delete old conflict ranges
				if let Err(err) = conn
					.execute(
						"DELETE FROM conflict_ranges where ts < now() - interval '10 seconds'",
						&[],
					)
					.await
				{
					tracing::error!(?err, "failed postgres gc task");
				}
			}
		});

		Ok(PostgresDatabaseDriver {
			pool: Arc::new(pool),
			max_retries: AtomicI32::new(100),
			gc_handle,
		})
	}
}

impl DatabaseDriver for PostgresDatabaseDriver {
	fn create_trx(&self) -> Result<Transaction> {
		// Pass the connection pool to the transaction driver
		Ok(Transaction::new(Arc::new(PostgresTransactionDriver::new(
			self.pool.clone(),
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

impl Drop for PostgresDatabaseDriver {
	fn drop(&mut self) {
		self.gc_handle.abort();
	}
}
