use std::{
	net::Ipv4Addr,
	path::Path,
	time::{self, Duration},
};

use anyhow::*;
use indoc::indoc;
use notify::{
	event::{AccessKind, AccessMode},
	Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use pegboard::protocol;
use serde::Deserialize;
use sqlx::{
	migrate::MigrateDatabase,
	sqlite::{SqlitePool, SqlitePoolOptions},
	Sqlite,
};
use tokio::{
	fs,
	sync::mpsc::{channel, Receiver},
};

use pegboard_config::{Addresses, Config};

pub mod sql;

const MAX_QUERY_RETRIES: usize = 16;
const QUERY_RETRY: Duration = Duration::from_millis(500);
const TXN_RETRY: Duration = Duration::from_millis(250);

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

pub async fn init_sqlite_schema(pool: &SqlitePool) -> Result<()> {
	// Attempt to use an existing connection
	let mut conn = if let Some(conn) = pool.try_acquire() {
		conn
	} else {
		// Create a new connection
		pool.acquire().await?
	};

	let settings = [
		// Set the journal mode to Write-Ahead Logging for concurrency
		"PRAGMA journal_mode = WAL",
		// Set synchronous mode to NORMAL for performance and data safety balance
		"PRAGMA synchronous = NORMAL",
		// Set busy timeout to 5 seconds to avoid "database is locked" errors
		"PRAGMA busy_timeout = 5000",
		// Enable foreign key constraint enforcement
		"PRAGMA foreign_keys = ON",
		// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
		"PRAGMA auto_vacuum = INCREMENTAL",
	];

	for setting in settings {
		sqlx::query(setting).execute(&mut *conn).await?;
	}

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS state (
			last_event_idx INTEGER NOT NULL,
			last_command_idx INTEGER NOT NULL,

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
		CREATE TABLE IF NOT EXISTS actors (
			actor_id BLOB PRIMARY KEY, -- UUID
			config BLOB NOT NULL,

			start_ts INTEGER NOT NULL,
			running_ts INTEGER,
			stop_ts INTEGER,
			exit_ts INTEGER,

			pid INTEGER,
			exit_code INTEGER
		) STRICT
		",
	))
	.execute(&mut *conn)
	.await?;

	sqlx::query(indoc!(
		"
		CREATE TABLE IF NOT EXISTS actor_ports (
			actor_id BLOB NOT NULL, -- UUID
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
		ON actor_ports(actor_id)
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

#[derive(Deserialize)]
struct ApiResponse {
	servers: Vec<ApiServer>,
}

#[derive(Deserialize)]
struct ApiServer {
	vlan_ip: Option<Ipv4Addr>,
}

pub async fn init_fdb_config(config: &Config) -> Result<()> {
	let ips = match &config.client.foundationdb.addresses {
		Addresses::Dynamic { fetch_endpoint } => reqwest::get(fetch_endpoint.clone())
			.await?
			.error_for_status()?
			.json::<ApiResponse>()
			.await?
			.servers
			.into_iter()
			.filter_map(|server| server.vlan_ip)
			.map(|vlan_ip| format!("{vlan_ip}:4500"))
			.collect::<Vec<_>>(),
		Addresses::Static(addresses) => addresses.clone(),
	};

	ensure!(!ips.is_empty(), "no fdb clusters found");

	let joined = ips
		.into_iter()
		.map(|x| x.to_string())
		.collect::<Vec<_>>()
		.join(",");

	fs::write(
		config.client.data_dir().join("fdb.cluster"),
		format!(
			"{cluster_description}:{cluster_id}@{joined}",
			cluster_description = config.client.foundationdb.cluster_description,
			cluster_id = config.client.foundationdb.cluster_id,
		),
	)
	.await?;

	Ok(())
}

pub async fn fetch_pull_addresses(config: &Config) -> Result<Vec<String>> {
	let mut addresses = match &config.client.images.pull_addresses {
		Addresses::Dynamic { fetch_endpoint } => reqwest::get(fetch_endpoint.clone())
			.await?
			.error_for_status()?
			.json::<ApiResponse>()
			.await?
			.servers
			.into_iter()
			.filter_map(|server| server.vlan_ip)
			.map(|vlan_ip| format!("{vlan_ip}:8080"))
			.collect::<Vec<_>>(),
		Addresses::Static(addresses) => addresses.clone(),
	};

	// Always sort the addresses so the list is deterministic
	addresses.sort();

	Ok(addresses)
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
					bail!("max sql retries: {err:?}");
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
