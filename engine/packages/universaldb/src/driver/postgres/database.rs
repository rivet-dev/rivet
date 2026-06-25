use std::{
	path::PathBuf,
	sync::{
		Arc,
		atomic::{AtomicI32, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result};
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use rivet_postgres_util::build_tls_config;
use tokio::task::JoinHandle;
use tokio_postgres_rustls::MakeRustlsConnect;
use url::Url;
use uuid::Uuid;

use crate::{
	RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, Erased},
	error::DatabaseError,
	transaction::TXN_TIMEOUT,
	utils::{MaybeCommitted, calculate_tx_retry_backoff},
};

use super::{
	listener::PgListener, resolver, shared::PostgresShared, transaction::PostgresTransactionDriver,
};

const GC_INTERVAL: Duration = Duration::from_secs(30);
/// Terminal and orphaned commit-request rows older than this are garbage collected. Must be well
/// beyond the longest a follower could spend awaiting a result, so a result is never deleted before
/// it is observed.
const COMMIT_ROW_MAX_AGE_SECS: i64 = 60;

#[derive(Clone, Debug)]
pub struct PostgresConfig {
	pub connection_string: String,
	pub ssl_config: Option<PostgresSslConfig>,
}

#[derive(Clone, Debug)]
pub struct PostgresSslConfig {
	pub ssl_root_cert_path: Option<PathBuf>,
	pub ssl_client_cert_path: Option<PathBuf>,
	pub ssl_client_key_path: Option<PathBuf>,
}

impl PostgresConfig {
	/// Create a new PostgreSQL configuration with sane defaults
	pub fn new(connection_string: String) -> Self {
		Self {
			connection_string,
			ssl_config: None,
		}
	}
}

pub struct PostgresDatabaseDriver {
	shared: Arc<PostgresShared>,
	max_retries: AtomicI32,
	resolver_handle: JoinHandle<()>,
	gc_handle: JoinHandle<()>,
}

impl PostgresDatabaseDriver {
	/// Create a new PostgreSQL driver with custom configuration
	pub async fn new_with_config(config: PostgresConfig) -> Result<Self> {
		tracing::debug!(
			connection_string = ?config.connection_string,
			"creating PostgresDatabaseDriver"
		);

		let ssl_disabled = if let Ok(url) = Url::parse(&config.connection_string) {
			url.query_pairs()
				.any(|(k, v)| k == "sslmode" && v == "disable")
		} else {
			false
		};

		let pool = Self::build_pool(&config, ssl_disabled)?;

		// Initialize the schema (idempotent).
		{
			let conn = pool
				.get()
				.await
				.context("failed to get connection from postgres pool")?;
			Self::init_schema(&conn).await?;
		}

		// Unique per-process node id (no hyphens) used to name this node's NOTIFY channels. Kept
		// short so `udb_commit_<node_id>` stays within Postgres's 63-byte identifier limit.
		let node_id = Uuid::new_v4().simple().to_string();

		let listener = PgListener::new(
			config.connection_string.clone(),
			ssl_disabled,
			config
				.ssl_config
				.as_ref()
				.and_then(|c| c.ssl_root_cert_path.clone()),
			config
				.ssl_config
				.as_ref()
				.and_then(|c| c.ssl_client_cert_path.clone()),
			config
				.ssl_config
				.as_ref()
				.and_then(|c| c.ssl_client_key_path.clone()),
		);

		let shared = PostgresShared::new(pool, node_id, listener);

		// Every node runs the resolver; only the elected leader drains the commit queue.
		let resolver_handle = resolver::spawn(shared.clone());

		let gc_handle = Self::spawn_gc(shared.clone());

		Ok(PostgresDatabaseDriver {
			shared,
			max_retries: AtomicI32::new(100),
			resolver_handle,
			gc_handle,
		})
	}

	fn build_pool(config: &PostgresConfig, ssl_disabled: bool) -> Result<Pool> {
		let mut pool_config = Config::new();
		pool_config.url = Some(config.connection_string.clone());
		pool_config.pool = Some(PoolConfig {
			max_size: 64,
			..Default::default()
		});
		pool_config.manager = Some(ManagerConfig {
			recycling_method: RecyclingMethod::Fast,
		});

		if ssl_disabled {
			pool_config
				.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
				.context("failed to create postgres connection pool")
		} else {
			let tls_config = build_tls_config(
				config
					.ssl_config
					.as_ref()
					.and_then(|c| c.ssl_root_cert_path.as_ref()),
				config
					.ssl_config
					.as_ref()
					.and_then(|c| c.ssl_client_cert_path.as_ref()),
				config
					.ssl_config
					.as_ref()
					.and_then(|c| c.ssl_client_key_path.as_ref()),
			)?;
			pool_config
				.create_pool(Some(Runtime::Tokio1), MakeRustlsConnect::new(tls_config))
				.context("failed to create postgres connection pool")
		}
	}

	async fn init_schema(conn: &deadpool_postgres::Client) -> Result<()> {
		// Durable latest-value store.
		conn.batch_execute(
			"CREATE TABLE IF NOT EXISTS kv (
				key BYTEA PRIMARY KEY,
				value BYTEA NOT NULL
			);

			CREATE TABLE IF NOT EXISTS udb_lease (
				id              INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
				epoch           BIGINT NOT NULL,
				leader_addr     TEXT   NOT NULL,
				durable_version BIGINT NOT NULL DEFAULT 0,
				expires_at      TIMESTAMPTZ NOT NULL
			);

			CREATE SEQUENCE IF NOT EXISTS udb_version_seq AS BIGINT
				START WITH 1 INCREMENT BY 1 MINVALUE 1;

			CREATE TABLE IF NOT EXISTS udb_commit_requests (
				id             BIGSERIAL PRIMARY KEY,
				epoch          BIGINT NOT NULL,
				read_version   BIGINT NOT NULL,
				payload        BYTEA  NOT NULL,
				reply_channel  TEXT   NOT NULL,
				status         TEXT   NOT NULL DEFAULT 'pending',
				commit_version BIGINT,
				created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
			);

			CREATE INDEX IF NOT EXISTS udb_commit_requests_pending
				ON udb_commit_requests (id) WHERE status = 'pending';",
		)
		.await
		.context("failed to initialize postgres schema")?;

		Ok(())
	}

	fn spawn_gc(shared: Arc<PostgresShared>) -> JoinHandle<()> {
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(GC_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;

				let conn = match shared.pool.get().await {
					Ok(conn) => conn,
					Err(err) => {
						tracing::debug!(?err, "failed to get connection for commit gc");
						continue;
					}
				};

				if let Err(err) = conn
					.execute(
						"DELETE FROM udb_commit_requests
						 WHERE created_at < now() - ($1::bigint * interval '1 second')",
						&[&COMMIT_ROW_MAX_AGE_SECS],
					)
					.await
				{
					tracing::error!(?err, "failed postgres commit-queue gc");
				}
			}
		})
	}
}

impl DatabaseDriver for PostgresDatabaseDriver {
	fn create_txn(&self) -> Result<Transaction> {
		Ok(Transaction::new(Arc::new(PostgresTransactionDriver::new(
			self.shared.clone(),
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
				let tx = self.create_txn()?;
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

	fn txn_retry_limit(&self, limit: i32) -> Result<()> {
		self.max_retries.store(limit, Ordering::SeqCst);
		Ok(())
	}
}

impl Drop for PostgresDatabaseDriver {
	fn drop(&mut self) {
		// Abort the resolver so a dropped node stops renewing its lease; the lease then expires and
		// another node can take over. Without this a dropped leader would renew its lease forever.
		self.resolver_handle.abort();
		self.gc_handle.abort();
	}
}
