use std::{collections::HashMap, sync::Arc, time::Duration};

use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{
		SqliteAutoVacuum, SqliteConnectOptions, SqliteConnection, SqlitePoolOptions,
		SqliteSynchronous,
	},
	Executor, Sqlite,
};
use tokio::sync::Mutex;

use crate::Error;

pub type SqlitePool = sqlx::SqlitePool;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Key {
	key: String,
	read_only: bool,
}

#[derive(Clone)]
pub struct SqlitePoolManager {
	// TODO: Somehow remove old pools
	pools: Arc<Mutex<HashMap<Key, SqlitePool>>>,
}

impl SqlitePoolManager {
	pub fn new() -> Self {
		SqlitePoolManager {
			pools: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	/// Get or creates an sqlite pool for the given key
	pub async fn get(&self, key: &str, read_only: bool) -> Result<SqlitePool, Error> {
		let mut pools_guard = self.pools.lock().await;

		let key = Key {
			key: key.to_string(),
			read_only,
		};

		let pool = if let Some(pool) = pools_guard.get(&key) {
			pool.clone()
		} else {
			// TODO: Hardcoded for testing
			let db_url = format!("sqlite:///var/lib/rivet-sqlite/{}.db", key.key);

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

			let opts = db_url
				.parse::<SqliteConnectOptions>()
				.map_err(Error::BuildSqlx)?
				.read_only(read_only)
				// Set synchronous mode to NORMAL for performance and data safety balance
				.synchronous(SqliteSynchronous::Normal)
				// Set busy timeout to 5 seconds to avoid "database is locked" errors
				.busy_timeout(Duration::from_secs(5))
				// Enable foreign key constraint enforcement
				.foreign_keys(true)
				// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
				.auto_vacuum(SqliteAutoVacuum::Incremental);

			let pool = SqlitePoolOptions::new()
				.max_lifetime_jitter(Duration::from_secs(90))
				// Open connection immediately on startup
				.min_connections(1)
				.after_connect(|conn, _meta| Box::pin(async move { setup_pragma(conn).await }))
				.connect_with(opts)
				.await
				.map_err(Error::BuildSqlx)?;

			tracing::debug!(?key, "sqlite connected");

			pools_guard.insert(key, pool.clone());

			pool
		};

		Ok(pool)
	}
}

async fn setup_pragma(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
	let settings = [
		// NOTE: sqlx doesnt seem to have a WAL2 option so we set it with a PRAGMA query
		// Set the journal mode to Write-Ahead Logging 2 for concurrency
		"PRAGMA journal_mode = WAL2",
	];

	conn.execute(settings.join("; ").as_str()).await?;

	Ok(())
}
