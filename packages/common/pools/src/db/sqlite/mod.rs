use dirs;
use fdb_util::{prelude::*, SERIALIZABLE};
use foundationdb::{self as fdb, options::StreamingMode, tuple::Subspace, FdbBindingError};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
struct SqliteState {
	total_changes: i64,
	schema_version: i64,
}
use futures_util::TryStreamExt;
use global_error::{ensure, unwrap, GlobalResult};
use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteSynchronous},
	ConnectOptions, Sqlite,
};
use std::io;
use std::{
	collections::HashMap,
	fmt::Debug,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
	time::Duration,
};
use tokio::sync::oneshot;
use tokio::{
	sync::{broadcast, Mutex},
	time::Instant,
};

use crate::{Error, FdbPool};

mod keys;

#[cfg(test)]
mod tests;

const GC_INTERVAL: Duration = Duration::from_secs(1);
const POOL_TTL: Duration = Duration::from_secs(15);
const CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

#[derive(Clone)]
pub enum SqliteConnType {
	/// There should only be one writer per DB.
	///
	/// The file will be loaded on the the machine and accessible until evicted.
	///
	/// Make sure to evict once done with the database.
	Writer { auto_snapshot: bool },
	/// Reads the database from FDB.
	///
	/// This is not cached in any way. This does not need to be evicted.
	Reader,
}

impl SqliteConnType {
	fn is_writer(&self) -> bool {
		match self {
			SqliteConnType::Writer { .. } => true,
			SqliteConnType::Reader => false,
		}
	}

	fn is_reader(&self) -> bool {
		match self {
			SqliteConnType::Writer { .. } => false,
			SqliteConnType::Reader => true,
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum SqliteFdbError {
	#[error("mismatched chunk {key_idx}, expected {chunk_count}")]
	MismatchedChunk { chunk_count: usize, key_idx: usize },
}

/// Pool that's returned to users of SQLite.
///
/// By default automatically snapshots database on drop.
/// This can be disabled with `set_auto_snapshot(false)`.
pub struct SqlitePool {
	entry: SqliteEntryHandle,
	/// Used to notify future when this is dropped.
	/// Only Some if auto_snapshot is enabled.
	_snapshot_task: Option<oneshot::Sender<()>>,
}

impl SqlitePool {
	fn new(entry: SqliteEntryHandle) -> SqlitePool {
		// Only spawn snapshot task if auto_snapshot is enabled
		let snapshot_task = match entry.conn_type {
			SqliteConnType::Writer {
				auto_snapshot: true,
			} => {
				let entry = entry.clone();
				let (snapshot_tx, snapshot_rx) = oneshot::channel();
				tokio::task::spawn(async move {
					// Wait for drop signal
					let _ = snapshot_rx.await;

					// tracing::debug!("sqlite writer dropped, auto-snapshotting");
					// match entry.snapshot().await {
					// 	Ok(_) => {}
					// 	Err(err) => tracing::error!(
					// 		?err,
					// 		"failed to snapshot on drop, will attempt snapshotting again when gc'd"
					// 	),
					// }
				});
				Some(snapshot_tx)
			}
			_ => None,
		};

		SqlitePool {
			entry,
			_snapshot_task: snapshot_task,
		}
	}
	pub async fn snapshot(&self) -> GlobalResult<bool> {
		self.entry.snapshot().await
	}

	pub async fn debug_db_size(&self) -> GlobalResult<u64> {
		self.entry.debug_db_size().await
	}
}

// HACK: Implement mock methods to make this act like an SQLite pool so it can be used with the SQL
// macros.
impl SqlitePool {
	pub fn try_acquire(&self) -> Option<tokio::sync::MutexGuard<'_, sqlx::SqliteConnection>> {
		self.entry.conn.try_lock().ok()
	}

	pub async fn acquire(
		&self,
	) -> Result<tokio::sync::MutexGuard<'_, sqlx::SqliteConnection>, sqlx::Error> {
		Ok(self.entry.conn.lock().await)
	}

	pub async fn conn(
		&self,
	) -> Result<tokio::sync::MutexGuard<'_, sqlx::SqliteConnection>, sqlx::Error> {
		Ok(self.entry.conn.lock().await)
	}
}

pub type SqliteEntryHandle = Arc<SqliteEntry>;

/// SQLite pool that's loaded on this machine.
pub struct SqliteEntry {
	key_packed: KeyPacked,
	conn: Arc<Mutex<sqlx::SqliteConnection>>,
	conn_type: SqliteConnType,
	db_path: PathBuf,
	storage: SqliteStorage,

	/// Last time this pool was accessed (either by `get` or a ref was dropped, meaning the query
	/// ended)
	///
	/// Only used for writers
	last_access: Mutex<Instant>,

	/// State from the last snapshot
	///
	/// Only used for writers
	snapshotted_state: Mutex<SqliteState>,

	manager: SqlitePoolManagerHandleWeak,

	/// Used to notify future when this is dropped to clean up the DB file
	_cleanup_task: Option<oneshot::Sender<()>>,
}

impl SqliteEntry {
	/// Snapshots the database to FDB. Should be called any time you need to be able to restore
	/// from the DB.
	async fn snapshot(self: &Arc<Self>) -> GlobalResult<bool> {
		let manager = unwrap!(self.manager.upgrade(), "manager is dropped");
		manager.snapshot_sqlite_db(self).await
	}

	/// Returns the size of the database file in bytes
	async fn debug_db_size(self: &Arc<Self>) -> GlobalResult<u64> {
		let metadata = tokio::fs::metadata(&self.db_path)
			.await
			.map_err(Error::Io)?;
		Ok(metadata.len())
	}
}

/// DB key in packed form. This is not the full FDB key, this is the DB name segment in DbDataKey.
type KeyPacked = Arc<Vec<u8>>;

pub type SqlitePoolManagerHandle = Arc<SqlitePoolManager>;
pub type SqlitePoolManagerHandleWeak = Weak<SqlitePoolManager>;

#[derive(Clone)]
enum SqliteStorage {
	Local { path: PathBuf },
	FoundationDb,
}

pub struct SqlitePoolManager {
	/// Pools that are writers.
	writer_pools: Arc<Mutex<HashMap<KeyPacked, SqliteEntryHandle>>>,
	shutdown: broadcast::Sender<()>,
	fdb: Option<FdbPool>,
	storage: SqliteStorage,
	subspace: Subspace,
}

impl SqlitePoolManager {
	pub async fn new(fdb: Option<FdbPool>) -> Result<SqlitePoolManagerHandle, Error> {
		let pools = Arc::new(Mutex::new(HashMap::new()));
		let (shutdown, _) = broadcast::channel(1);
		let shutdown_rx = shutdown.subscribe();

		let storage = if std::env::var("_RIVET_POOL_SQLITE_FORCE_LOCAL").map_or(false, |x| x == "1")
		{
			// Use platform-specific data directory
			let path = dirs::data_local_dir()
				.ok_or_else(|| {
					Error::Io(io::Error::new(
						io::ErrorKind::NotFound,
						"Could not determine local data directory",
					))
				})?
				.join("rivet-sqlite");

			// Ensure the directory exists
			tokio::fs::create_dir_all(&path)
				.await
				.map_err(|x| Error::Global(x.into()))?;

			SqliteStorage::Local { path }
		} else {
			SqliteStorage::FoundationDb
		};

		let pools_clone = pools.clone();
		let manager = Arc::new(SqlitePoolManager {
			writer_pools: pools_clone,
			shutdown,
			fdb: fdb.clone(),
			storage,
			subspace: Subspace::all().subspace(&("rivet", "sqlite")),
		});

		tokio::task::spawn(manager.clone().manager_gc_loop(shutdown_rx));

		Ok(manager)
	}

	async fn read_from_fdb(&self, key_packed: KeyPacked, db_path: &Path) -> GlobalResult<()> {
		let db_data_subspace = self
			.subspace
			.subspace(&keys::DbDataKey::new(key_packed.clone()));

		let fdb = unwrap!(self.fdb.as_ref());
		let (data, chunks) = fdb
			.run(|tx, _mc| {
				let db_data_subspace = db_data_subspace.clone();
				async move {
					// Fetch all chunks
					let mut data_stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&db_data_subspace).into()
						},
						SERIALIZABLE,
					);

					// Aggregate data
					let mut buf = Vec::new();
					let mut chunk_count = 0;
					while let Some(entry) = data_stream.try_next().await? {
						// Parse key
						let key = self
							.subspace
							.unpack::<keys::DbDataChunkKey>(entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						// Validate chunk
						if chunk_count != key.chunk {
							return Err(FdbBindingError::CustomError(
								SqliteFdbError::MismatchedChunk {
									chunk_count,
									key_idx: key.chunk,
								}
								.into(),
							));
						}
						chunk_count += 1;

						// Write to buffer
						buf.extend(entry.value());
					}

					Ok::<_, FdbBindingError>((buf, chunk_count))
				}
			})
			.await?;

		if chunks > 0 {
			tracing::debug!(?chunks, data_len = ?data.len(), "loaded database from fdb");
			tokio::fs::write(db_path, data).await.map_err(Error::Io)?;
		} else {
			tracing::debug!("no sqlite db exists");
		}

		Ok(())
	}

	/// Get or creates an sqlite pool for the given key
	///
	/// IMPORTANT: Do not hold a reference to this value for an extended period of time. We use
	/// this function call to determine when to GC a pool.
	pub async fn get(
		self: &Arc<Self>,
		key: impl TuplePack + Debug,
		conn_type: SqliteConnType,
	) -> Result<SqlitePool, Error> {
		let mut writer_pools_guard = self.writer_pools.lock().await;

		let key_packed = Arc::new(key.pack_to_vec());

		// Check if pool already exists
		if conn_type.is_writer() {
			if let Some(entry) = writer_pools_guard.get_mut(&key_packed) {
				*entry.last_access.lock().await = Instant::now();
				return Ok(SqlitePool::new(entry.clone()));
			}
		}

		// Load from FDB if enabled
		let hex_key_str = hex::encode(&*key_packed);
		let db_path = match &self.storage {
			SqliteStorage::Local { path } => {
				// Determine the persistent location of this database
				let db_path = path.join(format!("{hex_key_str}.db"));

				db_path
			}
			SqliteStorage::FoundationDb => {
				// Generate temporary file location so multiple readers don't clobber each other
				let db_path = std::env::temp_dir()
					.join(format!("rivet-sqlite-{hex_key_str}-{}.db", Uuid::new_v4()));

				// If the database exists, write to the path
				self.read_from_fdb(key_packed.clone(), &db_path)
					.await
					.map_err(Error::Global)?;

				db_path
			}
		};

		let db_url = format!("sqlite://{}", db_path.display());
		tracing::debug!(?key, ?db_url, "sqlite connecting");

		// Create database if needed
		if !Sqlite::database_exists(&db_url)
			.await
			.map_err(Error::BuildSqlx)?
		{
			tracing::debug!(?db_url, "creating sqlite database");

			Sqlite::create_database(&db_url)
				.await
				.map_err(Error::BuildSqlx)?;
		}

		// Connect to database
		//
		// We don't need a connection pool since we only have one reader/writer at a time
		let conn_raw = db_url
			.parse::<SqliteConnectOptions>()
			.map_err(Error::BuildSqlx)?
			.read_only(conn_type.is_reader())
			// Set busy timeout to 5 seconds to avoid "database is locked" errors
			.busy_timeout(Duration::from_secs(5))
			// Enable foreign key constraint enforcement
			.foreign_keys(true)
			// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
			.auto_vacuum(SqliteAutoVacuum::Incremental)
			// Disable WAL for snapshotting
			//
			// Truncate is faster than Delete
			.journal_mode(sqlx::sqlite::SqliteJournalMode::Truncate)
			// Force all operations to be flushed to disk
			//
			// This impacts performance, but is required in order for snapshot to work
			.synchronous(sqlx::sqlite::SqliteSynchronous::Full)
			.connect()
			.await
			.map_err(Error::BuildSqlx)?;

		let conn = Arc::new(Mutex::new(conn_raw));

		tracing::debug!(?key, "sqlite connected");

		let cleanup_task = match &self.storage {
			SqliteStorage::FoundationDb => {
				let db_path = db_path.clone();
				let (cleanup_tx, cleanup_rx) = oneshot::channel();

				tokio::task::spawn(async move {
					let _ = cleanup_rx.await;

					if let Err(err) = tokio::fs::remove_file(&db_path).await {
						tracing::warn!(
							?err,
							?db_path,
							"failed to remove temporary sqlite db file on drop"
						);
					}
				});

				Some(cleanup_tx)
			}
			SqliteStorage::Local { .. } => None,
		};

		let pool = Arc::new(SqliteEntry {
			key_packed: key_packed.clone(),
			conn: conn.clone(),
			conn_type: conn_type.clone(),
			db_path,
			storage: self.storage.clone(),
			last_access: Mutex::new(Instant::now()),
			snapshotted_state: Mutex::new(SqliteState {
				total_changes: 0,
				schema_version: 0,
			}),
			manager: Arc::downgrade(self),
			_cleanup_task: cleanup_task,
		});

		if conn_type.is_writer() {
			writer_pools_guard.insert(key_packed.clone(), pool.clone());
		}

		Ok(SqlitePool::new(pool.clone()))
	}

	/// Evicts a database from the pool and snapshots it if needed
	pub async fn evict(self: &Arc<Self>, key: impl TuplePack) -> Result<(), Error> {
		let key_packed = Arc::new(key.pack_to_vec());

		let mut writer_pools_guard = self.writer_pools.lock().await;

		self.evict_database_inner(&key_packed, &mut writer_pools_guard)
			.await
			.map_err(Error::Global)?;

		Ok(())
	}

	/// If the database is loaded, then force a snapshot, or wait for existing snapshot to finish
	/// writing.
	pub async fn flush(self: &Arc<Self>, key: impl TuplePack) -> Result<(), Error> {
		let key_packed = Arc::new(key.pack_to_vec());

		let writer_pools_guard = self.writer_pools.lock().await;
		if let Some(entry) = writer_pools_guard.get(&key_packed) {
			self.snapshot_sqlite_db(entry)
				.await
				.map_err(Error::Global)?;
		}

		Ok(())
	}
}

impl SqlitePoolManager {
	/// Inner implementation of database eviction that handles the actual removal from the pool
	async fn evict_database_inner(
		&self,
		key: &KeyPacked,
		writer_pools_guard: &mut tokio::sync::MutexGuard<'_, HashMap<KeyPacked, SqliteEntryHandle>>,
	) -> GlobalResult<()> {
		tracing::debug!("evicting sqlite database");

		let Some(entry) = writer_pools_guard.get(key) else {
			return Ok(());
		};

		// Attempt to snapshot before removing
		self.snapshot_sqlite_db(entry).await?;

		// Note: The database file will be deleted when the SqliteEntry is dropped

		// Remove from the pools map
		writer_pools_guard.remove(key);

		Ok(())
	}

	/// GC loop for SqlitePoolManager
	async fn manager_gc_loop(self: Arc<Self>, mut shutdown: broadcast::Receiver<()>) {
		let mut interval = tokio::time::interval(GC_INTERVAL);

		loop {
			tokio::select! {
				_ = interval.tick() => {},
				Ok(_) = shutdown.recv() => {
					tracing::debug!("shutting down sqlite pool manager");
					break;
				}
			}

			// Anything last used before this instant will be removed
			let expire_ts = Instant::now() - POOL_TTL;

			// Remove pools
			{
				let mut writer_pools_guard = self.writer_pools.lock().await;
				let mut removed = 0;

				// Find entries to remove
				let mut to_remove = Vec::new();
				for (k, v) in writer_pools_guard.iter() {
					// TODO: Figure out how to do this without a mutex
					if *v.last_access.lock().await <= expire_ts {
						// Validate that this is the only reference to the database
						let ref_count = Arc::strong_count(&v);
						if ref_count == 1 {
							to_remove.push(k.clone());
						} else {
							tracing::warn!(?ref_count, ?k, "sqlite pool is expired and should have no references, but still has references");
						}
					}
				}

				// Evict each entry
				for key in to_remove {
					match self
						.evict_database_inner(&key, &mut writer_pools_guard)
						.await
					{
						Ok(_) => {
							removed += 1;
						}
						Err(err) => {
							tracing::error!(
								?err,
								?key,
								"failed to evict sqlite db, will retry later"
							);
						}
					}
				}

				tracing::debug!(?removed, total = ?writer_pools_guard.len(), "gc sqlite pools");
			}
		}
	}

	/// Snapshots the current state of a SQLite database to FDB.
	///
	/// This will acquire an exclusive lock on the database to ensure consistency.
	///
	/// We can do this because we don't use WAL (since we don't need concurrent readers/writers).
	///
	/// We don't use `VACUUM FULL` because it requires significant overhead to execute frequently,
	/// which we don't need since we don't use a WAL.
	///
	/// We don't use the `.backup` command (or `sqlite3_backup_*`) because it still has some
	/// overhead.
	///
	/// Returns `true` if wrote a snapshot.
	async fn snapshot_sqlite_db(&self, entry: &SqliteEntryHandle) -> GlobalResult<bool> {
		// Only run if snapshotting required
		let SqliteStorage::FoundationDb = self.storage else {
			return Ok(false);
		};

		let mut conn = entry.conn.lock().await;
		let mut snapshotted_state = entry.snapshotted_state.lock().await;

		tracing::debug!("snapshotting sqlite database");

		// Start an IMMEDIATE transaction to prevent other write transactions
		sqlx::query("BEGIN IMMEDIATE TRANSACTION;")
			.execute(&mut *conn)
			.await
			.map_err(|e| Error::BuildSqlx(e))?;

		// Use a Result to track if we need to roll back
		let snapshot_result = self
			.snapshot_sqlite_db_inner(entry, &mut *conn, *snapshotted_state)
			.await;

		// Always roll back the transaction since we only used it for consistent reading
		let rollback_result = sqlx::query("ROLLBACK;")
			.execute(&mut *conn)
			.await
			.map_err(Error::BuildSqlx);

		if let Err(rollback_err) = &rollback_result {
			tracing::error!(?rollback_err, "Failed to rollback transaction");
		}

		// Update state if snapshot was successful
		if let Ok(Some(current_state)) = &snapshot_result {
			*snapshotted_state = *current_state;
		}

		// Return the snapshot result, not the rollback result
		snapshot_result.map(|x| x.is_some())
	}

	/// Writes the database to storage. This is executed during an SQLite transaction that blocks
	/// all other queries.
	///
	/// Returns `true` if total_changes() if changed.
	async fn snapshot_sqlite_db_inner(
		&self,
		entry: &SqliteEntryHandle,
		conn: &mut sqlx::SqliteConnection,
		snapshotted_state: SqliteState,
	) -> GlobalResult<Option<SqliteState>> {
		let fdb = unwrap!(self.fdb.as_ref());

		// Get current state
		let current_state = SqliteState {
			total_changes: sqlx::query_scalar("SELECT total_changes()")
				.fetch_one(&mut *conn)
				.await
				.map_err(Error::BuildSqlx)?,
			schema_version: sqlx::query_scalar("PRAGMA schema_version")
				.fetch_one(&mut *conn)
				.await
				.map_err(Error::BuildSqlx)?,
		};

		// Compare with last snapshot state
		ensure!(
			snapshotted_state.total_changes <= current_state.total_changes,
			"total_changes() went down"
		);
		if snapshotted_state == current_state {
			tracing::debug!("no changes detected, skipping sqlite database snapshot");
			return Ok(None);
		}

		tracing::debug!(
			?snapshotted_state,
			?current_state,
			"detected changes in sqlite database"
		);

		// Read the database file
		let data = tokio::fs::read(&entry.db_path).await.map_err(Error::Io)?;

		// Write to FDB
		fdb.run(|tx, _mc| {
			let data = data.clone();
			let key_packed = entry.key_packed.clone();
			let subspace = self.subspace.clone();
			async move {
				// Clear previous data
				let db_data_subspace = subspace.subspace(&keys::DbDataKey::new(key_packed.clone()));
				tx.clear_subspace_range(&db_data_subspace);

				// Write chunks
				for (idx, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
					let chunk_key = keys::DbDataChunkKey {
						db_name_segment: key_packed.clone(),
						chunk: idx,
					};
					tx.set(&subspace.pack(&chunk_key), chunk);
				}

				Ok(())
			}
		})
		.await?;

		Ok(Some(current_state))
	}

	// TODO: This version should be faster but it's not
	//async fn snapshot_sqlite_db_inner(&self, entry: &SqliteEntryHandle) -> GlobalResult<()> {
	//	let fdb = unwrap!(self.fdb.as_ref());
	//	let db_path = self.build_sqlite_file_path(entry.key_packed.clone());
	//
	//	// Write to FDB while holding the transaction
	//	fdb.run(|tx, _mc| {
	//		let key_packed = entry.key_packed.clone();
	//		let subspace = self.subspace.clone();
	//		let db_path = db_path.clone();
	//
	//		async move {
	//			// Open file for reading
	//			let mut file = tokio::fs::File::open(&db_path).await.map_err(|e| fdb::FdbBindingError::CustomError(e.into()))?;
	//			let mut buffer = vec![0; CHUNK_SIZE];
	//			let mut chunk_idx = 0;
	//
	//			// Clear previous data
	//			let db_data_subspace = subspace.subspace(&keys::DbDataKey::new(key_packed.clone()));
	//			tx.clear_subspace_range(&db_data_subspace);
	//
	//			// Read and write file in chunks
	//			loop {
	//				let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer)
	//					.await
	//					.map_err(|e| fdb::FdbBindingError::CustomError(e.into()))?;
	//
	//				if n == 0 {
	//					break; // End of file
	//				}
	//
	//				let chunk_key = keys::DbDataChunkKey {
	//					db_name_segment: key_packed.clone(),
	//					chunk: chunk_idx,
	//				};
	//
	//				// Only write the bytes that were actually read
	//				tx.set(&subspace.pack(&chunk_key), &buffer[..n]);
	//
	//				chunk_idx += 1;
	//			}
	//
	//			Ok(())
	//		}
	//	})
	//	.await?;
	//
	//	Ok(())
	//}
}

impl Drop for SqlitePoolManager {
	fn drop(&mut self) {
		// Ignore send errors - receivers may already be dropped
		let _ = self.shutdown.send(());
	}
}
