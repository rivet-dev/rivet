use std::{
	hash::{DefaultHasher, Hasher},
	path::Path,
	result::Result::{Err, Ok},
	time::{self, Duration},
};

use anyhow::*;
use futures_util::Stream;
use indoc::indoc;
use notify::{
	event::{AccessKind, AccessMode},
	Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use pegboard::protocol;
use pegboard_config::Config;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha12Rng;
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
use url::Url;
use uuid::Uuid;

use crate::ctx::Ctx;

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

			PRIMARY KEY (actor_id, generation)
		) STRICT
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

/// Generates a list of address URLs for a given build ID, with deterministic shuffling.
///
/// This function accepts a build ID and returns an array of URLs, including both
/// the seeded shuffling and the fallback address (if provided).
pub async fn get_image_addresses(
	ctx: &Ctx,
	image_id: Uuid,
	image_artifact_url_stub: &str,
	image_fallback_artifact_url: Option<&str>,
) -> Result<Vec<String>> {
	// Get hash from image id
	let mut hasher = DefaultHasher::new();
	hasher.write(image_id.as_bytes());
	let hash = hasher.finish();

	let mut rng = ChaCha12Rng::seed_from_u64(hash);

	// Shuffle based on hash
	let mut addresses = ctx
		.pull_addr_handler
		.addresses(ctx.config())
		.await?
		.iter()
		.map(|addr| {
			Ok(Url::parse(&format!("{addr}{}", image_artifact_url_stub))
				.context("failed to build artifact url")?
				.to_string())
		})
		.collect::<Result<Vec<_>>>()?;
	addresses.shuffle(&mut rng);

	// Add fallback url to the end if one is set
	if let Some(fallback_artifact_url) = image_fallback_artifact_url {
		addresses.push(fallback_artifact_url.to_string());
	}

	ensure!(
		!addresses.is_empty(),
		"no artifact urls available (no pull addresses nor fallback)"
	);

	Ok(addresses)
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

/// Deterministically shuffles a list of available ATS URL's to download the image from based on the image
// ID and attempts to download from each URL until success.
pub async fn fetch_image_stream(
	ctx: &Ctx,
	image_id: Uuid,
	image_artifact_url_stub: &str,
	image_fallback_artifact_url: Option<&str>,
) -> Result<impl Stream<Item = reqwest::Result<bytes::Bytes>>> {
	let addresses = get_image_addresses(
		ctx,
		image_id,
		image_artifact_url_stub,
		image_fallback_artifact_url,
	)
	.await?;

	let mut iter = addresses.into_iter();
	while let Some(artifact_url) = iter.next() {
		// Log the full URL we're attempting to download from
		tracing::info!(?image_id, %artifact_url, "attempting to download image");

		match reqwest::get(&artifact_url)
			.await
			.and_then(|res| res.error_for_status())
		{
			Ok(res) => {
				tracing::info!(?image_id, %artifact_url, "successfully downloading image");
				return Ok(res.bytes_stream());
			}
			Err(err) => {
				tracing::warn!(
					?image_id,
					%artifact_url,
					%err,
					"failed to download image"
				);
			}
		}
	}

	bail!("artifact url could not be resolved");
}

pub async fn prewarm_image(ctx: &Ctx, image_id: Uuid, image_artifact_url_stub: &str) {
	// Log full URL for prewarm operation
	let prewarm_url = format!("{}/{}", image_artifact_url_stub, image_id);
	tracing::info!(?image_id, %prewarm_url, "prewarming image");

	match fetch_image_stream(ctx, image_id, image_artifact_url_stub, None).await {
		Ok(_) => tracing::info!(?image_id, %prewarm_url, "prewarm complete"),
		Err(_) => tracing::warn!(
			?image_id,
			%prewarm_url,
			"prewarm failed, artifact url could not be resolved"
		),
	}
}
