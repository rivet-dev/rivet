use std::{
	path::{Path, PathBuf},
	time::{self, Duration},
};

use anyhow::*;
use futures_util::StreamExt;
use indoc::indoc;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{SqlitePool, SqlitePoolOptions},
	Sqlite,
};
use tokio::sync::mpsc::{channel, Receiver};
use tokio::{
	fs::{self, File},
	io::AsyncWriteExt,
};
use url::Url;

const MAX_QUERY_RETRIES: usize = 16;
const QUERY_RETRY: Duration = Duration::from_millis(500);
const TXN_RETRY: Duration = Duration::from_millis(250);

pub fn var(name: &str) -> Result<String> {
	std::env::var(name).context(name.to_string())
}

pub async fn init_working_dir(working_path: &Path) -> Result<()> {
	if fs::metadata(&working_path).await.is_err() {
		bail!("working dir `{}` does not exist", working_path.display());
	}

	// Create containers dir
	match fs::create_dir(working_path.join("containers")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /containers dir in working dir")?,
	}

	// Create db dir
	match fs::create_dir(working_path.join("db")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /db dir in working dir")?,
	}

	// Create bin dir
	match fs::create_dir(working_path.join("bin")).await {
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
		x => x.context("failed to create /bin dir in working dir")?,
	}

	Ok(())
}

pub async fn download_file(url: &str, file_path: &Path) -> Result<()> {
	// Fix tokio/anyhow macro bug
	use std::result::Result::{Err, Ok};

	// Create file and start request
	let (mut file, response) = tokio::try_join!(
		async {
			File::create(file_path)
				.await
				.map_err(Into::<anyhow::Error>::into)
		},
		async { reqwest::get(url).await.map_err(Into::<anyhow::Error>::into) }
	)?;

	let mut stream = response.error_for_status()?.bytes_stream();

	// Write from stream to file
	while let Some(chunk) = stream.next().await {
		file.write_all(&chunk?).await?;
	}

	anyhow::Ok(())
}

// Get `UUID/job-runner` from URL (HOST/s3-cache/aws/BUCKET/job-runner/UUID/job-runner)
pub fn get_s3_path_stub(url: &Url, with_uuid: bool) -> Result<PathBuf> {
	let path_segments = url.path_segments().context("bad container runner url")?;
	let path_stub = path_segments
		.rev()
		.take(if with_uuid { 2 } else { 1 })
		.collect::<Vec<_>>()
		.into_iter()
		.rev()
		.collect::<PathBuf>();

	Ok(path_stub)
}

pub async fn init_sqlite_db(db_url: &str) -> Result<()> {
	if !Sqlite::database_exists(db_url).await? {
		Sqlite::create_database(db_url).await?;
	}

	Ok(())
}

pub async fn build_sqlite_pool(db_url: &str) -> Result<SqlitePool> {
	SqlitePoolOptions::new()
		.connect(db_url)
		.await
		.map_err(Into::into)
}

// TODO: Replace with migrations
pub async fn init_sqlite_schema(pool: &SqlitePool) -> Result<()> {
	// Attempt to use an existing connection
	let mut conn = if let Some(conn) = pool.try_acquire() {
		conn
	} else {
		// Create a new connection
		pool.acquire().await?
	};

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS state (
			last_event_idx INTEGER NOT NULL,
			last_command_idx INTEGER NOT NULL
		)
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		INSERT INTO state
		VALUES (0, 0)
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
		)
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
		)
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS containers (
			container_id TEXT PRIMARY KEY, -- UUID
			config BLOB NOT NULL,

			start_ts INTEGER NOT NULL,
			running_ts INTEGER,
			stop_ts INTEGER,
			exit_ts INTEGER,

			pid INTEGER,
			exit_code INTEGER
		)
		",
	))
	.execute(&mut *conn)
	.await?;

	Ok(())
}

/// Executes queries and explicitly handles retry errors.
pub async fn query<'a, F, Fut, T>(mut cb: F) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = std::result::Result<T, sqlx::Error>> + 'a,
	T: 'a,
{
	let mut i = 0;

	loop {
		match cb().await {
			std::result::Result::Ok(x) => return Ok(x),
			std::result::Result::Err(err) => {
				use sqlx::Error::*;

				if i > MAX_QUERY_RETRIES {
					bail!("max sql retries: {err}");
				}
				i += 1;

				match &err {
					// Retry transaction errors immediately
					Database(db_err)
						if db_err
							.message()
							.contains("TransactionRetryWithProtoRefreshError") =>
					{
						tracing::info!(message=%db_err.message(), "transaction retry");
						tokio::time::sleep(TXN_RETRY).await;
					}
					// Retry internal errors with a backoff
					Database(_) | Io(_) | Tls(_) | Protocol(_) | PoolTimedOut | PoolClosed
					| WorkerCrashed => {
						tracing::info!(?err, "query retry");
						tokio::time::sleep(QUERY_RETRY).await;
					}
					// Throw error
					_ => return Err(err.into()),
				}
			}
		}
	}
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
			// `Create` event is received in wait_for_creation
			let _ = tx.blocking_send(res);
		},
		Config::default().with_poll_interval(Duration::from_secs(2)),
	)?;

	Ok((watcher, rx))
}

pub async fn wait_for_creation<P: AsRef<Path>>(path: P) -> Result<()> {
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

		// Wait for creation of path
		if let EventKind::Create(_) = res.kind {
			if res.paths.iter().any(|p| p == path) {
				break;
			}
		}
	}

	Ok(())
}
