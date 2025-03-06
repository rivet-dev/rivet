use std::{collections::HashMap, ops::Deref, sync::Arc, time::Duration};

use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{
		SqliteAutoVacuum, SqliteConnectOptions, SqliteConnection, SqlitePoolOptions,
		SqliteSynchronous,
	},
	Executor, Sqlite,
};
use tokio::sync::Mutex;

use crate::db::atomic_instant::AtomicInstant;

use crate::Error;

const GC_INTERVAL: Duration = Duration::from_secs(1);
const POOL_TTL: Duration = Duration::from_secs(15);

/// Pool wrapper that tracks when it was dropped to automatically update the last used on
/// `SqlitePoolEntry` in order to know when to GC the pool.
pub struct SqlitePool {
	pool: sqlx::SqlitePool,

	// Holds a reference to the last access time
	last_access: Arc<AtomicInstant>,
}

impl Deref for SqlitePool {
	type Target = sqlx::SqlitePool;

	fn deref(&self) -> &Self::Target {
		&self.pool
	}
}

impl Drop for SqlitePool {
	fn drop(&mut self) {
		// Update last access
		self.last_access.store_now();
	}
}

struct SqlitePoolEntry {
	pool: sqlx::SqlitePool,

	/// Last time this pool was accessed
	last_access: Arc<AtomicInstant>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Key {
	key: String,
	read_only: bool,
}

#[derive(Clone)]
pub struct SqlitePoolManager {
	pools: Arc<Mutex<HashMap<Key, SqlitePoolEntry>>>,
}

impl SqlitePoolManager {
	pub fn new() -> Self {
		let pools = Arc::new(Mutex::new(HashMap::new()));
		tokio::task::spawn(Self::start(pools.clone()));
		SqlitePoolManager { pools }
	}

	async fn start(pools: Arc<Mutex<HashMap<Key, SqlitePoolEntry>>>) {
		let mut interval = tokio::time::interval(GC_INTERVAL);

		loop {
			interval.tick().awati;

			// Anything last used before this instant will be removed
			let expire_ts = Instant::now() - POOL_TTL;

			// Remove pools
			{
				let pools_guard = pools.lock().await;
				let mut removed = 0;
				pools_guard.retain(|_k, v| {
					if v.last_access.load() > expire_ts {
						true
					} else {
						removed += 1;
						false
					}
				});
			}
		}
	}

	/// Get or creates an sqlite pool for the given key
	pub async fn get(&self, key: &str, read_only: bool) -> Result<SqlitePool, Error> {
		let mut pools_guard = self.pools.lock().await;

		let key = Key {
			key: key.to_string(),
			read_only,
		};

		let pool = if let Some(entry) = pools_guard.get(&key) {
			entry.last_access = Instant::now();
			entry.pool.clone()
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

			pools_guard.insert(
				key,
				SqlitePoolEntry {
					pool: pool.clone(),
					last_access: Arc::new(AtomicInstant::new()),
				},
			);

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
