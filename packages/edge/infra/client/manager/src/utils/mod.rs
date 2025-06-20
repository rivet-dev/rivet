use std::{
	path::Path,
	result::Result::{Err, Ok},
	time::{self, Duration},
};

use anyhow::*;
use indoc::indoc;
use notify::{
	event::{AccessKind, AccessMode},
	Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use pegboard::protocol;
use pegboard_config::Config;
use sql::SqlitePoolExt;
use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous},
	Executor, Sqlite, SqlitePool,
};
use tokio::{
	fs,
	sync::mpsc::{channel, Receiver},
};

pub mod libc;
pub mod sql;

pub async fn init_dir(config: &Config) -> Result<()> {
	let data_dir = config.client.data_dir();

	if fs::metadata(&data_dir).await.is_err() {
		bail!("data dir `{}` does not exist", data_dir.display());
	}

	if config.client.runner.flavor == protocol::ClientFlavor::Container
		&& fs::metadata(&config.client.runner.container_runner_binary_path())
			.await
			.is_err()
	{
		bail!(
			"container runner binary `{}` does not exist",
			config
				.client
				.runner
				.container_runner_binary_path()
				.display()
		);
	}

	if config.client.runner.flavor == protocol::ClientFlavor::Isolate
		&& fs::metadata(&config.client.runner.isolate_runner_binary_path())
			.await
			.is_err()
	{
		bail!(
			"isolate runner binary `{}` does not exist",
			config.client.runner.isolate_runner_binary_path().display()
		);
	}

	// Create actors dir
	match fs::create_dir(data_dir.join("actors")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /actors dir in data dir")?,
	}

	// Create images dir
	match fs::create_dir(data_dir.join("images")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /images dir in data dir")?,
	}

	// Create runner dir
	match fs::create_dir(data_dir.join("runner")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /runner dir in data dir")?,
	}

	// Create db dir
	match fs::create_dir(data_dir.join("db")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /db dir in data dir")?,
	}

	Ok(())
}

pub async fn init_sqlite_db(config: &Config) -> Result<SqlitePool> {
	let sqlite_db_url = format!(
		"sqlite://{}",
		config
			.client
			.data_dir()
			.join("db")
			.join("database.db")
			.display()
	);

	if !Sqlite::database_exists(&sqlite_db_url).await? {
		Sqlite::create_database(&sqlite_db_url).await?;
	}

	// Connect to sqlite db
	let pool = build_sqlite_pool(&sqlite_db_url).await?;
	init_sqlite_schema(&pool).await?;

	Ok(pool)
}

async fn build_sqlite_pool(db_url: &str) -> Result<SqlitePool> {
	let opts = db_url
		.parse::<SqliteConnectOptions>()?
		// Set synchronous mode to NORMAL for performance and data safety balance
		.synchronous(SqliteSynchronous::Normal)
		// Set busy timeout to 5 seconds to avoid "database is locked" errors
		.busy_timeout(Duration::from_secs(5))
		// Enable foreign key constraint enforcement
		.foreign_keys(true)
		// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
		.auto_vacuum(SqliteAutoVacuum::Incremental);

	let pool = SqlitePoolOptions::new()
		.after_connect(|conn, _meta| {
			Box::pin(async move {
				// NOTE: sqlx doesn't seem to have a WAL2 option so we set it with a PRAGMA query
				conn.execute("PRAGMA journal_mode = WAL2").await?;

				Ok(())
			})
		})
		// Open connection immediately on startup
		.min_connections(1)
		.connect_with(opts)
		.await?;

	Ok(pool)
}

async fn init_sqlite_schema(pool: &SqlitePool) -> Result<()> {
	let mut conn = pool.conn().await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS state (
			last_event_idx INTEGER NOT NULL,
			last_command_idx INTEGER NOT NULL,
			last_workflow_id BLOB, -- UUID

			isolate_runner_pid INTEGER,

			-- Keeps this table having one row
			_persistence INTEGER UNIQUE NOT NULL DEFAULT TRUE -- BOOLEAN
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		INSERT INTO state (last_event_idx, last_command_idx)
		VALUES (-1, -1)
		ON CONFLICT DO NOTHING
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS events (
			idx INTEGER NOT NULL UNIQUE,
			payload BLOB NOT NULL,
			create_ts INTEGER NOT NULL
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS commands (
			idx INTEGER NOT NULL UNIQUE,
			payload BLOB NOT NULL,
			ack_ts INTEGER NOT NULL
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS images_cache (
			image_id BLOB NOT NULL, -- UUID
			size INTEGER NOT NULL,

			last_used_ts INTEGER NOT NULL,
			download_complete_ts INTEGER,

			PRIMARY KEY (image_id)
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS actors (
			actor_id BLOB NOT NULL, -- UUID
			generation INTEGER NOT NULL,
			config BLOB NOT NULL,

			start_ts INTEGER NOT NULL,
			running_ts INTEGER,
			stop_ts INTEGER,
			exit_ts INTEGER,

			pid INTEGER,
			exit_code INTEGER,

			-- Also exists in the config column but this is for indexing
			image_id BLOB NOT NULL, -- UUID

			PRIMARY KEY (actor_id, generation)
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE INDEX IF NOT EXISTS actors_image_id_idx
		ON actors(image_id)
		WHERE stop_ts IS NULL
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS actor_ports (
			actor_id BLOB NOT NULL, -- UUID
			generation INT NOT NULL,
			port INT NOT NULL,
			protocol INT NOT NULL, -- protocol::TransportProtocol

			delete_ts INT
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE INDEX IF NOT EXISTS actor_ports_id_idx
		ON actor_ports(actor_id, generation)
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE UNIQUE INDEX IF NOT EXISTS actor_ports_unique_idx
		ON actor_ports(port, protocol)
		WHERE delete_ts IS NULL
		",
	))
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub fn now() -> i64 {
	time::SystemTime::now()
		.duration_since(time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}

/// Creates an async file watcher.
fn async_watcher() -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
	let (tx, rx) = channel(1);

	// Automatically select the best implementation for your platform.
	let watcher = RecommendedWatcher::new(
		move |res| {
			let tx = tx.clone();

			// Send event. We ignore the result because the watcher and channel are dropped after the first
			// `Create` event is received in wait_for_write
			let _ = tx.blocking_send(res);
		},
		notify::Config::default().with_poll_interval(Duration::from_secs(2)),
	)?;

	Ok((watcher, rx))
}

pub async fn wait_for_write<P: AsRef<Path>>(path: P) -> Result<()> {
	let path = path.as_ref();
	let (mut watcher, mut rx) = async_watcher()?;

	// Watch parent
	watcher.watch(
		path.parent().context("path has no parent")?,
		RecursiveMode::NonRecursive,
	)?;

	// File already exists
	if fs::metadata(&path).await.is_ok() {
		return Ok(());
	}

	while let Some(res) = rx.recv().await {
		let res = res?;

		// Wait for data to be written to path
		if let EventKind::Access(AccessKind::Close(AccessMode::Write)) = res.kind {
			if res.paths.iter().any(|p| p == path) {
				break;
			}
		}
	}

	Ok(())
}

/// Recursively copy a directory from source to destination.
pub async fn copy_dir_all<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<()> {
	let src = src.as_ref();
	let dst = dst.as_ref();

	if !src.is_dir() {
		return Err(anyhow!("source is not a directory: {}", src.display()));
	}

	if !dst.exists() {
		fs::create_dir_all(dst).await?;
	} else if !dst.is_dir() {
		return Err(anyhow!(
			"destination exists but is not a directory: {}",
			dst.display()
		));
	}

	let mut read_dir = fs::read_dir(src).await?;

	while let Some(entry) = read_dir.next_entry().await? {
		let entry_path = entry.path();
		let file_name = entry.file_name();
		let dst_path = dst.join(file_name);

		if entry_path.is_dir() {
			Box::pin(copy_dir_all(entry_path, dst_path)).await?;
		} else {
			fs::copy(entry_path, dst_path).await?;
		}
	}

	Ok(())
}

/// Calculates the total size of a folder in bytes.
pub async fn total_dir_size<P: AsRef<Path>>(path: P) -> Result<u64> {
	let path = path.as_ref();

	ensure!(path.is_dir(), "path is not a directory: {}", path.display());

	let mut total_size = 0;
	let mut read_dir = fs::read_dir(path).await.context("failed to read dir")?;

	while let Some(entry) = read_dir.next_entry().await.transpose() {
		let entry = match entry {
			Ok(entry) => entry,
			Err(err) => {
				tracing::debug!(?err, "failed to read entry");
				continue;
			}
		};
		let entry_path = entry.path();

		if entry_path.is_dir() {
			match Box::pin(total_dir_size(entry_path)).await {
				Ok(size) => total_size += size,
				Err(err) => {
					tracing::debug!(?err, p=?entry.path().display(), "failed to calculate size for directory");
					continue;
				}
			}
		} else {
			match fs::metadata(entry_path).await {
				Ok(metadata) => total_size += metadata.len(),
				Err(err) => {
					tracing::debug!(?err, p=?entry.path().display(), "failed to get metadata for file");
					continue;
				}
			}
		}
	}

	Ok(total_size)
}
