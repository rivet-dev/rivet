use std::{
	fmt::Debug,
	io::{self, Read, Write},
	path::{Path, PathBuf},
	sync::{Arc, Weak},
	time::Duration,
};

use dirs;
use fdb_util::{prelude::*, SERIALIZABLE};
use foundationdb::{self as fdb, options::StreamingMode, FdbBindingError};
use futures_util::{StreamExt, TryStreamExt};
use global_error::{bail, ext::AssertionError, unwrap, GlobalResult};
use rivet_util::future::CustomInstrumentExt;
use sqlx::{
	migrate::MigrateDatabase,
	pool::PoolConnection,
	sqlite::{
		SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqliteLockingMode,
		SqliteSynchronous,
	},
	Sqlite,
};
use tokio::sync::{oneshot, OnceCell, RwLock};
use tokio::{io::AsyncReadExt, sync::broadcast, time::Instant};
use tracing::Instrument;
use uuid::Uuid;

use crate::{metrics, Error, FdbPool};

pub mod keys;

#[cfg(test)]
mod tests;

const GC_INTERVAL: Duration = Duration::from_secs(5);
const POOL_TTL: Duration = Duration::from_secs(15);
const CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

#[derive(Debug, Clone)]
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

impl std::fmt::Display for SqliteConnType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SqliteConnType::Writer { .. } => write!(f, "writer"),
			SqliteConnType::Reader => write!(f, "reader"),
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum SqliteFdbError {
	#[error("mismatched chunk {key_idx}, expected {chunk_count}")]
	MismatchedChunk { chunk_count: usize, key_idx: usize },
}

/// SQLite pool that's loaded on this machine.
pub struct SqliteWriterEntry {
	key_packed: KeyPacked,
	conn_type: SqliteConnType,

	/// The underlying pool.
	///
	/// This is stored as a OnceCell since we need to be able to insert the SqliteEntry in to the
	/// HashMap before the connection is initiated.
	pool_once: OnceCell<SqlitePool>,

	/// Last time this pool was accessed (either by `get` or a ref was dropped, meaning the query
	/// ended)
	///
	/// Only used for writers
	last_access: RwLock<Instant>,

	manager: SqlitePoolManagerHandleWeak,
}

impl SqliteWriterEntry {
	fn new(
		key_packed: KeyPacked,
		conn_type: SqliteConnType,
		manager: SqlitePoolManagerHandleWeak,
	) -> SqliteWriterEntry {
		SqliteWriterEntry {
			key_packed,
			conn_type,

			pool_once: OnceCell::new(),

			last_access: RwLock::new(Instant::now()),
			manager,
		}
	}

	#[tracing::instrument(name = "sqlite_writer_pool", skip_all)]
	async fn pool(&self) -> Result<&SqlitePool, Error> {
		self.pool_once
			.get_or_try_init(|| {
				async {
					let manager = self.manager.upgrade().ok_or_else(|| {
						Error::Global(
							AssertionError::Panic {
								message: "manager is dropped".into(),
								location: global_error::location!(),
							}
							.into(),
						)
					})?;

					SqlitePoolInner::new(self.key_packed.clone(), self.conn_type.clone(), manager)
						.await
				}
				.instrument(tracing::info_span!("pool_connect"))
			})
			.await
	}
}

/// DB key in packed form. This is not the full FDB key, this is the DB name segment in DbDataKey.
///
/// Stored in an `Arc` since this is frequently copied around.
pub type KeyPacked = Arc<Vec<u8>>;

pub type SqlitePoolManagerHandle = Arc<SqlitePoolManager>;
pub type SqlitePoolManagerHandleWeak = Weak<SqlitePoolManager>;

#[derive(Clone)]
enum SqliteStorage {
	Local { path: PathBuf },
	FoundationDb { path: PathBuf },
}

pub struct SqlitePoolManager {
	/// Writer pools are kept in memory. Reader pools are one-off temporary SQLite databases.
	writer_pools: papaya::HashMap<KeyPacked, SqliteWriterEntry>,
	shutdown: broadcast::Sender<()>,
	fdb: Option<FdbPool>,
	storage: SqliteStorage,
	subspace: fdb_util::Subspace,
}

// MARK: Public methods
impl SqlitePoolManager {
	#[tracing::instrument(name = "sqlite_pool_manager_new", skip_all)]
	pub async fn new(fdb: Option<FdbPool>) -> Result<SqlitePoolManagerHandle, Error> {
		let (shutdown, _) = broadcast::channel(1);
		let shutdown_rx = shutdown.subscribe();

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

		let storage = if std::env::var("_RIVET_POOL_SQLITE_FORCE_LOCAL").map_or(false, |x| x == "1")
		{
			SqliteStorage::Local { path }
		} else {
			SqliteStorage::FoundationDb { path }
		};

		let manager = Arc::new(SqlitePoolManager {
			writer_pools: papaya::HashMap::new(),
			shutdown,
			fdb: fdb.clone(),
			storage,
			subspace: fdb_util::Subspace::new(&(RIVET, SQLITE)),
		});

		tokio::task::spawn(manager.clone().manager_gc_loop(shutdown_rx));

		Ok(manager)
	}

	/// Get or creates an sqlite pool for the given key
	///
	/// IMPORTANT: Do not hold a reference to this value for an extended period of time. We use
	/// this function call to determine when to GC a pool.
	#[tracing::instrument(name = "sqlite_get", skip_all)]
	pub async fn get(
		self: &Arc<Self>,
		key: impl TuplePack + Debug,
		conn_type: SqliteConnType,
	) -> Result<SqlitePool, Error> {
		let start_instant = Instant::now();

		let key_packed = Arc::new(key.pack_to_vec());
		let conn_type_str = &conn_type.to_string();
		let mut did_insert = false;

		// Check if pool already exists
		let pool = if conn_type.is_writer() {
			let pinned = self.writer_pools.pin_owned();

			// Insert entry
			let entry = pinned.get_or_insert_with(key_packed.clone(), || {
				did_insert = true;

				// NOTE: Database will be loaded lazily on first call of `.conn()`. This is not
				// for performance, this is because we need it to be a `OnceCell` in order to
				// use it with the async hashmap.
				SqliteWriterEntry::new(key_packed.clone(), conn_type.clone(), Arc::downgrade(self))
			});

			{
				*entry.last_access.write().await = Instant::now();
			}

			entry.pool().await?.clone()
		} else {
			SqlitePoolInner::new(key_packed.clone(), conn_type, self.clone()).await?
		};

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::SQLITE_GET_POOL_DURATION
			.with_label_values(&[conn_type_str, &did_insert.to_string()])
			.observe(dt);

		Ok(pool)
	}

	/// Evicts databases from the pool and snapshots them if needed
	#[tracing::instrument(name = "sqlite_evict", skip_all)]
	pub async fn evict<T: TuplePack>(self: &Arc<Self>, keys: Vec<T>) -> Result<(), Error> {
		let keys_packed: Vec<KeyPacked> = keys
			.into_iter()
			.map(|key| Arc::new(key.pack_to_vec()))
			.collect();

		self.evict_with_key(&keys_packed)
			.await
			.map_err(Error::Global)?;

		Ok(())
	}

	/// If the databases are loaded, then force a snapshot, or wait for existing snapshot to finish
	/// writing.
	#[tracing::instrument(name = "sqlite_flush", skip_all)]
	pub async fn flush<T: TuplePack>(
		self: &Arc<Self>,
		keys: Vec<T>,
		vacuum: bool,
	) -> Result<(), Error> {
		let keys_packed: Vec<KeyPacked> = keys
			.into_iter()
			.map(|key| Arc::new(key.pack_to_vec()))
			.collect();

		self.snapshot_with_key(&keys_packed, vacuum, false)
			.await
			.map_err(Error::Global)?;

		Ok(())
	}
}

// MARK: Private helpers
impl SqlitePoolManager {
	fn db_path(&self, key_packed: &KeyPacked) -> PathBuf {
		let hex_key_str = hex::encode(&**key_packed);

		match &self.storage {
			// Determine the persistent location of this database
			SqliteStorage::Local { path } => path.join(format!("{hex_key_str}.db")),
			// Generate temporary file location so multiple readers don't clobber each other
			SqliteStorage::FoundationDb { path } => {
				path.join(format!("rivet-sqlite-{hex_key_str}-{}.db", Uuid::new_v4()))
			}
		}
	}

	/// Inner implementation of database eviction that handles the actual removal from the pool
	#[tracing::instrument(name = "sqlite_evict_with_key", skip_all)]
	async fn evict_with_key(&self, keys_packed: &[KeyPacked]) -> GlobalResult<()> {
		if keys_packed.is_empty() {
			return Ok(());
		}

		for key_packed in keys_packed {
			tracing::debug!(key=?hex::encode(&**key_packed), "evicting sqlite database");
		}

		// Attempt to snapshot all databases in a single call
		self.snapshot_with_key(keys_packed, true, false).await?;

		// Remove all databases from the pools map
		// Do this after snapshotting since we don't want to remove the db if the snapshot failed.
		// If the snapshot failed, it will attempt to snapshot again on GC.
		for key_packed in keys_packed {
			if let Some(entry) = self.writer_pools.pin_owned().remove(key_packed) {
				// NOTE: papaya does not immediately release memory of entries when you call `.remove`.
				// This means the pool will stick around for some time, so we close the pool and delete files
				// after removing to ensure we don't have too many open connections.
				tokio::join!(
					async {
						match entry.pool().await {
							Ok(pool) => pool.close().await,
							Err(err) => {
								tracing::debug!(?key_packed, ?err, "failed to get pool for closing")
							}
						}
					},
					clear_db_files(&self.storage, self.db_path(&key_packed)),
				);
			}
		}

		Ok(())
	}

	/// Snapshots the current state of SQLite databases to FDB.
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
	/// Returns `true` if wrote at least one snapshot.
	#[tracing::instrument(name = "sqlite_snapshot_with_key", skip_all)]
	async fn snapshot_with_key(
		&self,
		keys_packed: &[KeyPacked],
		vacuum: bool,
		ensure_exists: bool,
	) -> GlobalResult<bool> {
		if keys_packed.is_empty() {
			return Ok(false);
		}

		// Only run if snapshotting required
		let SqliteStorage::FoundationDb { .. } = self.storage else {
			return Ok(false);
		};

		let start_instant = Instant::now();

		// Process each database connection in parallel using a stream
		let db_data_to_snapshot = futures_util::stream::iter(keys_packed.iter().cloned())
			.map(|key_packed| {
				async move {
					let hex_key = hex::encode(&**key_packed);

					// Acquire the pool
					let pool = if let Some(x) = self
						.writer_pools
						.pin()
						.get(&key_packed)
						.map(|v| v.pool_once.get().cloned())
					{
						x
					} else {
						if ensure_exists {
							bail!("attempting to snapshot database that's not loaded");
						} else {
							tracing::debug!(key=?hex_key, "skipping snapshot, database is not loaded");
							return GlobalResult::Ok(None);
						}
					};

					let pool = match pool {
						Some(pool) => pool,
						None => return Ok(None), // Pool will be None if it has not been initiated yet
					};

					tracing::debug!(key=?hex_key, "snapshotting sqlite database");

					let mut conn = pool.conn().await?;

					if vacuum {
						// NOTE: Must vacuum before checkpoint so the vacuum applies to the main db file
						sqlx::query("PRAGMA incremental_vacuum(1);")
							.execute(&mut *conn)
							.instrument(tracing::info_span!("vacuum"))
							.await?;
					}

					// Flush WAL journal
					sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
						.execute(&mut *conn)
						.instrument(tracing::info_span!("flush_wal"))
						.await?;

					// Stream the database file and compress it
					let mut compressed_data = Vec::new();
					let file = tokio::fs::File::open(&pool.db_path)
						.await
						.map_err(Error::Io)?;
					let mut reader = tokio::io::BufReader::new(file);
					let mut encoder = lz4_flex::frame::FrameEncoder::new(&mut compressed_data);

					async {
						let mut buffer = [0u8; 16 * 1024]; // 16 KiB
						loop {
							let bytes_read = reader.read(&mut buffer).await.map_err(Error::Io)?;
							if bytes_read == 0 {
								break;
							}
							encoder
								.write_all(&buffer[..bytes_read])
								.map_err(Error::Io)?;
						}
						encoder.finish().map_err(Error::Lz4)?;

						Result::<_, Error>::Ok(())
					}
					.instrument(tracing::info_span!("compress"))
					.await?;

					// 3 MiB
					if compressed_data.len() > 3 * 1024 * 1024 {
						metrics::SQLITE_LARGE_DB
							.with_label_values(&[&hex_key])
							.set(compressed_data.len().try_into().unwrap_or(i64::MAX));
					}

					Ok(Some((key_packed, Arc::new(compressed_data))))
				}
			})
			.buffer_unordered(32)
			.try_filter_map(|result| async move { Ok(result) })
			.try_collect::<Vec<_>>()
			.in_current_span()
			.await?;

		// If no databases need to be snapshotted, return early
		if db_data_to_snapshot.is_empty() {
			return Ok(false);
		}

		// Write to FDB in a single transaction
		let fdb = unwrap!(self.fdb.as_ref());
		fdb.run(|tx, _mc| {
			let db_data_to_snapshot = db_data_to_snapshot.clone();
			let subspace = self.subspace.clone();
			async move {
				for (key_packed, data) in &db_data_to_snapshot {
					// Clear previous data
					let db_data_subspace =
						subspace.subspace(&keys::DbDataKey::new(key_packed.clone()));
					tx.clear_subspace_range(&db_data_subspace);
					let compressed_db_data_subspace =
						subspace.subspace(&keys::CompressedDbDataKey::new(key_packed.clone()));
					tx.clear_subspace_range(&compressed_db_data_subspace);

					// Write chunks
					for (idx, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
						let chunk_key = keys::CompressedDbDataChunkKey {
							db_name_segment: key_packed.clone(),
							chunk: idx,
						};

						tx.set(&subspace.pack(&chunk_key), chunk);
					}
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("snapshot_sqlite_write_tx"))
		.await?;

		let dt = start_instant.elapsed().as_secs_f64();
		let total_data_size = db_data_to_snapshot
			.iter()
			.map(|(_, data)| data.len())
			.sum::<usize>() as f64;

		// Update state if write was successful
		for (key_packed, data) in db_data_to_snapshot {
			let hex_key = hex::encode(&**key_packed);

			// Because this was batch processed we don't know the rate for each individual key, just estimate
			// by calculating the size ratio
			let ratio = data.len() as f64 / total_data_size;
			metrics::SQLITE_UPLOAD_DB_RATE
				.with_label_values(&[&hex_key])
				.set(data.len() as f64 / (dt * ratio));
		}

		Ok(true)
	}

	/// GC loop for SqlitePoolManager
	#[tracing::instrument(skip_all)]
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

			metrics::SQLITE_POOL_SIZE.reset();

			// Find expired entries to remove
			//
			// We do this by collecting keys instead of using `retain` or `prune` since we only
			// want to remove the entry if it's successfully snapshotted. If it's not, it should be
			// retained in the map until successfully snapshotted.
			let mut to_remove = Vec::new();
			let mut total_count = 0;
			for (k, v) in self.writer_pools.pin_owned().iter() {
				metrics::SQLITE_POOL_SIZE
					.with_label_values(&[&v.conn_type.to_string()])
					.inc();

				total_count += 1;
				let ts = { *v.last_access.read().await };

				if ts <= expire_ts {
					if let Some(pool) = v.pool_once.get() {
						// Validate that this is the only reference to the database
						let ref_count = Arc::strong_count(pool);
						if ref_count == 1 {
							to_remove.push(k.clone());
						} else {
							tracing::warn!(?ref_count, ?k, "sqlite pool is expired and should have no references, but still has references");
						}
					} else {
						tracing::warn!(?k, "sqlite pool is expired but conn was never acquired");
						to_remove.push(k.clone());
					}
				}
			}

			// Evict each entry
			let mut removed = 0;
			for key in to_remove {
				match self.evict_with_key(&[key.clone()]).await {
					Ok(_) => {
						removed += 1;
					}
					Err(err) => {
						tracing::error!(?err, ?key, "failed to evict sqlite db, will retry later");
					}
				}
			}

			tracing::debug!(?removed, total=?total_count, "gc sqlite pools");
		}
	}
}

// MARK: FDB Helpers
impl SqlitePoolManager {
	/// Returns true if db exists, false if not.
	#[tracing::instrument(name = "sqlite_read_from_fdb", skip_all)]
	async fn read_from_fdb(&self, key_packed: KeyPacked, db_path: &Path) -> GlobalResult<bool> {
		let hex_key = hex::encode(&*key_packed);
		let fdb = unwrap!(self.fdb.as_ref());

		let start_instant = Instant::now();

		let (data, chunks) = fdb
			.run(|tx, _mc| {
				let key_packed = key_packed.clone();
				async move {
					let compressed_db_data_subspace = self
						.subspace
						.subspace(&keys::CompressedDbDataKey::new(key_packed.clone()));

					// Fetch all chunks
					let mut compressed_data_stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&compressed_db_data_subspace).into()
						},
						SERIALIZABLE,
					);

					// Aggregate data
					let mut buf = Vec::new();
					let mut chunk_count = 0;

					let mut compressed_data_buf = Vec::new();
					while let Some(entry) = compressed_data_stream.try_next().await? {
						// Parse key
						let key = self
							.subspace
							.unpack::<keys::CompressedDbDataChunkKey>(entry.key())
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
						compressed_data_buf.extend(entry.value());
					}

					let span = tracing::info_span!("decompress");
					let _guard = span.enter();
					// Decompress the data
					let mut decoder = lz4_flex::frame::FrameDecoder::new(&compressed_data_buf[..]);
					decoder
						.read_to_end(&mut buf)
						.map_err(Error::Io)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					drop(_guard);

					// If there is no compressed data, read from the uncompressed data (backwards compatibility)
					if chunk_count == 0 {
						let db_data_subspace = self
							.subspace
							.subspace(&keys::DbDataKey::new(key_packed.clone()));
						let mut data_stream = tx.get_ranges_keyvalues(
							fdb::RangeOption {
								mode: StreamingMode::WantAll,
								..(&db_data_subspace).into()
							},
							SERIALIZABLE,
						);

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
					}

					Ok((buf, chunk_count))
				}
			})
			.custom_instrument(tracing::info_span!("read_from_fdb_tx"))
			.await?;

		if chunks > 0 {
			let data_len = data.len();
			tracing::debug!(key=?hex_key, ?chunks, ?data_len, "loaded database from fdb");

			tokio::fs::write(db_path, data).await.map_err(Error::Io)?;

			let dt = start_instant.elapsed();
			metrics::SQLITE_DOWNLOAD_DB_RATE
				.with_label_values(&[&hex_key])
				.set(data_len as f64 / dt.as_secs_f64());

			Ok(true)
		} else {
			tracing::debug!(key=?hex_key, "db not found in fdb");

			Ok(false)
		}
	}
}

impl Drop for SqlitePoolManager {
	fn drop(&mut self) {
		// Ignore send errors - receivers may already be dropped
		let _ = self.shutdown.send(());
	}
}

pub type SqlitePool = Arc<SqlitePoolInner>;

pub struct SqlitePoolInner {
	key_packed: KeyPacked,
	inner: sqlx::SqlitePool,
	db_path: PathBuf,
	manager: SqlitePoolManagerHandle,

	/// Used to notify future when this is dropped.
	_drop_task: oneshot::Sender<()>,
}

impl SqlitePoolInner {
	async fn new(
		key_packed: KeyPacked,
		conn_type: SqliteConnType,
		manager: SqlitePoolManagerHandle,
	) -> Result<SqlitePool, Error> {
		let db_path = manager.db_path(&key_packed);
		let db_url = format!("sqlite://{}", db_path.display());

		// Load database
		match &manager.storage {
			SqliteStorage::Local { .. } => {
				if !Sqlite::database_exists(&db_url)
					.await
					.map_err(Error::BuildSqlx)?
				{
					tracing::debug!(?db_url, "sqlite database does not exist");
					if conn_type.is_reader() {
						return Err(Error::Global(
							AssertionError::Panic {
								message: "cannot open reader for database that doesn't exist"
									.into(),
								location: global_error::location!(),
							}
							.into(),
						));
					}
				} else {
					tracing::debug!(?db_url, "sqlite database already exists");
				}
			}
			SqliteStorage::FoundationDb { .. } => {
				// Read db from FDB
				let db_exists = manager
					.read_from_fdb(key_packed.clone(), &db_path)
					.await
					.map_err(Error::Global)?;

				// Create database if needed
				if !db_exists {
					tracing::debug!(?db_url, "sqlite database does not exist");
					if conn_type.is_reader() {
						return Err(Error::Global(
							AssertionError::Panic {
								message: "cannot open reader for database that doesn't exist"
									.into(),
								location: global_error::location!(),
							}
							.into(),
						));
					}
				} else {
					tracing::debug!(?db_url, "sqlite database already exists");
				}
			}
		};

		tracing::debug!(?db_url, "sqlite connecting");

		let opts = db_url
			.parse::<SqliteConnectOptions>()
			.map_err(Error::BuildSqlx)?;
		let pool_opts = sqlx::sqlite::SqlitePoolOptions::new()
			// The default connection timeout is too high
			.acquire_timeout(Duration::from_secs(60))
			.max_lifetime(Duration::from_secs(15 * 60))
			.max_lifetime_jitter(Duration::from_secs(90))
			// Remove connections after a while in order to reduce load after bursts
			.idle_timeout(Some(Duration::from_secs(10 * 60)));

		let (opts, pool_opts) = if conn_type.is_writer() {
			(
				opts.create_if_missing(true)
					// Enable foreign key constraint enforcement
					.foreign_keys(true)
					// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
					.auto_vacuum(SqliteAutoVacuum::Incremental)
					// Set synchronous mode to NORMAL for performance and data safety balance
					.synchronous(SqliteSynchronous::Normal)
					// Increases write performance
					.journal_mode(SqliteJournalMode::Wal)
					// Reduces file system operations
					.locking_mode(SqliteLockingMode::Exclusive),
				// Sqlite doesnt support more than 1 concurrent writer, will get "database is locked"
				pool_opts.min_connections(1).max_connections(1),
			)
		} else {
			(
				opts.read_only(true)
					// Enable foreign key constraint enforcement
					.foreign_keys(true)
					// Set synchronous mode to NORMAL for performance and data safety balance
					.synchronous(SqliteSynchronous::Normal),
				pool_opts
					// Open connections immediately on startup
					.min_connections(2)
					// Shouldn't have more than 64 connections to a single sqlite db
					.max_connections(64),
			)
		};

		// Create pool
		let res = pool_opts
			.connect_with(opts)
			.instrument(tracing::info_span!("sqlite_connect"))
			.await;

		let pool = match res {
			Ok(x) => x,
			Err(err) => {
				tracing::error!(
					?conn_type,
					?key_packed,
					?db_url,
					"failed to connect to sqlite"
				);
				return Err(Error::BuildSqlx(err));
			}
		};

		tracing::debug!(?db_url, "sqlite connected");

		// Create drop handle
		let (drop_tx, drop_rx) = oneshot::channel();
		tokio::spawn({
			let manager = manager.clone();
			let db_path = db_path.clone();

			async move {
				// Wait for drop signal
				let _ = drop_rx.await;

				clear_db_files(&manager.storage, db_path).await;
			}
			.instrument(tracing::info_span!("sqlite_pool_drop"))
		});

		Ok(Arc::new(SqlitePoolInner {
			key_packed,
			inner: pool,
			db_path,
			manager,
			_drop_task: drop_tx,
		}))
	}
}

impl SqlitePoolInner {
	// TODO: Doesn't need a result type
	#[tracing::instrument(name = "sqlite_pool_snapshot", skip_all)]
	pub async fn snapshot(&self, vacuum: bool) -> GlobalResult<bool> {
		match self
			.manager
			.snapshot_with_key(&[self.key_packed.clone()], vacuum, true)
			.await
		{
			Ok(x) => Ok(x),
			Err(err) => {
				tracing::error!(
					?err,
					"failed to snapshot, will attempt snapshotting again when gc'd"
				);
				Ok(false)
			}
		}
	}

	#[tracing::instrument(name = "sqlite_pool_evict", skip_all)]
	pub async fn evict(&self) -> GlobalResult<()> {
		self.manager
			.evict_with_key(&[self.key_packed.clone()])
			.await
	}
}

impl SqlitePoolInner {
	pub fn db_path(&self) -> &Path {
		&self.db_path
	}

	#[tracing::instrument(skip_all)]
	pub async fn conn(&self) -> Result<PoolConnection<Sqlite>, sqlx::Error> {
		// Attempt to use an existing connection
		if let Some(conn) = self.inner.try_acquire() {
			Ok(conn)
		} else {
			// Create a new connection
			self.inner.acquire().await
		}
	}
}

impl std::ops::Deref for SqlitePoolInner {
	type Target = sqlx::SqlitePool;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl std::ops::DerefMut for SqlitePoolInner {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

async fn clear_db_files(storage: &SqliteStorage, db_path: PathBuf) {
	// Remove file
	match storage {
		SqliteStorage::Local { .. } => {}
		SqliteStorage::FoundationDb { .. } => {
			let (res1, res2, res3) = tokio::join!(
				tokio::fs::remove_file(&db_path),
				tokio::fs::remove_file(format!("{}-shm", db_path.display())),
				tokio::fs::remove_file(format!("{}-wal", db_path.display())),
			);
			if let Err(err) = res1 {
				tracing::warn!(
					?err,
					?db_path,
					"failed to remove temporary sqlite db file on drop"
				);
			}

			if let Err(err) = res2 {
				tracing::debug!(
					?err,
					?db_path,
					"failed to remove temporary sqlite db shm file on drop"
				);
			}

			if let Err(err) = res3 {
				tracing::debug!(
					?err,
					?db_path,
					"failed to remove temporary sqlite db wal file on drop"
				);
			}
		}
	}
}
