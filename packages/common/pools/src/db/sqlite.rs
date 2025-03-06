use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::{StreamExt, TryStreamExt};
use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{SqliteConnectOptions, SqlitePoolOptions},
	Sqlite,
};
use tokio::sync::Mutex;

use crate::Error;

pub type SqlitePool = sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqlitePoolManager {
	// TODO: Somehow remove old pools
	pools: Arc<Mutex<HashMap<String, SqlitePool>>>,
}

impl SqlitePoolManager {
	pub fn new() -> Self {
		SqlitePoolManager {
			pools: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	/// Get or creates an sqlite pool for the given key
	pub async fn get(&self, key: &str) -> Result<SqlitePool, Error> {
		let mut pools_guard = self.pools.lock().await;

		let pool = if let Some(pool) = pools_guard.get(key) {
			pool.clone()
		} else {
			// TODO: Hardcoded for testing
			let db_url = format!("sqlite:///home/rivet/rivet-ee/oss/packages/common/chirp-workflow/core/tests/db/{key}.db");

			tracing::debug!(?key, "sqlite connecting");

			// Init if doesn't exist
			if !Sqlite::database_exists(&db_url)
				.await
				.map_err(Error::BuildSqlx)?
			{
				Sqlite::create_database(&db_url)
					.await
					.map_err(Error::BuildSqlx)?;
			}

			let opts: SqliteConnectOptions = db_url.parse().map_err(Error::BuildSqlx)?;

			let pool = SqlitePoolOptions::new()
				.max_lifetime_jitter(Duration::from_secs(90))
				// Open connection immediately on startup
				.min_connections(1)
				.connect_with(opts)
				.await
				.map_err(Error::BuildSqlx)?;

			// Run at the start of every connection
			setup_pragma(&pool).await.map_err(Error::BuildSqlx)?;

			pools_guard.insert(key.to_string(), pool.clone());

			tracing::debug!(?key, "sqlite connected");

			pool
		};

		Ok(pool)
	}
}

async fn setup_pragma(pool: &SqlitePool) -> Result<(), sqlx::Error> {
	// Has to be String instead of static str due to a weird compiler bug. This crate will compile just fine
	// but chirp-workflow will not and the error has nothing to do with this code
	let settings = [
		// Set the journal mode to Write-Ahead Logging 2 for concurrency
		"PRAGMA journal_mode = WAL2".to_string(),
		// Set synchronous mode to NORMAL for performance and data safety balance
		"PRAGMA synchronous = NORMAL".to_string(),
		// Set busy timeout to 5 seconds to avoid "database is locked" errors
		"PRAGMA busy_timeout = 5000".to_string(),
		// Enable foreign key constraint enforcement
		"PRAGMA foreign_keys = ON".to_string(),
		// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
		"PRAGMA auto_vacuum = INCREMENTAL".to_string(),
	];

	futures_util::stream::iter(settings)
		.map(|setting| {
			let pool = pool.clone();
			async move {
				// Attempt to use an existing connection
				let mut conn = if let Some(conn) = pool.try_acquire() {
					conn
				} else {
					// Create a new connection
					pool.acquire().await?
				};
				// Result::<_, sqlx::Error>::Ok(())

				sqlx::query(&setting).execute(&mut *conn).await
			}
		})
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
