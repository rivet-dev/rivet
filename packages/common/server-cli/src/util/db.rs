use std::{
	io::{Read, Write},
	path::Path,
	result::Result::Ok,
	str::FromStr,
	sync::Arc,
	time::Duration,
};

use anyhow::*;
use fdb_util::{prelude::*, SERIALIZABLE};
use foundationdb::{self as fdb, options::StreamingMode, FdbBindingError};
use futures_util::TryStreamExt;
use rivet_pools::db::sqlite::{keys, KeyPacked};
use serde_json::json;
use sqlx::sqlite::{
	SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqliteLockingMode, SqliteSynchronous,
};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

pub struct ShellQuery {
	pub svc: String,
	pub query: Option<String>,
}

pub struct ShellContext<'a> {
	pub queries: &'a [ShellQuery],
}

pub async fn redis_shell(config: rivet_config::Config, shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	for ShellQuery { svc, query } in queries {
		let server_config = config.server.as_ref().context("missing server")?;
		let redis_config = match svc.as_str() {
			"ephemeral" => &server_config.redis.ephemeral,
			"persistent" => &server_config.redis.persistent,
			_ => bail!("redis svc can only be ephemeral or persistent"),
		};

		tracing::info!(?svc, "connecting to redis");

		if query.is_some() {
			bail!("cannot pass query to redis shell at the moment");
		}

		let parsed_url = redis_config.url.clone();
		let hostname = parsed_url.host_str().context("missing hostname")?;
		let port = parsed_url.port().unwrap_or(6379);

		let mut cmd = std::process::Command::new("redis-cli");
		cmd.args(["-h", hostname, "-p", &port.to_string(), "-c", "--tls"]);
		if let Some(username) = &redis_config.username {
			cmd.arg("--user").arg(username);
		}
		if let Some(password) = &redis_config.password {
			cmd.arg("--pass").arg(password.read());
		}

		let ca_path = format!("/usr/local/share/ca-certificates/redis-{svc}-ca.crt");
		if Path::new(&ca_path).exists() {
			cmd.arg("--cacert").arg(&ca_path);
		}

		if let Some(password) = parsed_url.password() {
			cmd.env("REDISCLI_AUTH", password);
		}

		cmd.status()?;
	}

	Ok(())
}

pub async fn cockroachdb_shell(
	config: rivet_config::Config,
	shell_ctx: ShellContext<'_>,
) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	let server_config = config.server.as_ref().context("server not enabled")?;

	tracing::info!("connecting to cockroachdb");

	// Combine all queries into one command
	for ShellQuery {
		svc: db_name,
		query,
	} in queries
	{
		let mut parsed_url = server_config.cockroachdb.url.clone();
		parsed_url.set_path(&format!("/{}", db_name));

		let ca_path = "/usr/local/share/ca-certificates/crdb-ca.crt";
		if Path::new(&ca_path).exists() {
			parsed_url.set_query(Some(&format!("sslmode=verify-ca&sslrootcert={ca_path}")));
		} else {
			parsed_url.set_query(None);
		}
		parsed_url
			.set_username(&server_config.cockroachdb.username)
			.ok()
			.context("failed to set username")?;
		if let Some(password) = &server_config.cockroachdb.password {
			parsed_url
				.set_password(Some(password.read()))
				.ok()
				.context("failed to set password")?;
		}

		let db_url = parsed_url.to_string();

		let mut cmd = std::process::Command::new("psql");
		cmd.arg(&db_url);

		if let Some(query) = query {
			cmd.args(["-c", query]);
		}

		// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
		cmd.env("PGCLIENTENCODING", "utf-8");

		cmd.status()?;
	}

	Ok(())
}

pub async fn clickhouse_shell(
	config: rivet_config::Config,
	shell_ctx: ShellContext<'_>,
) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	let server_config = config.server.as_ref().context("server not enabled")?;

	tracing::info!("connecting to clickhouse");

	for ShellQuery {
		svc: db_name,
		query,
	} in queries
	{
		let clickhouse_config = server_config
			.clickhouse
			.as_ref()
			.context("clickhouse disabled")?;
		let parsed_url = clickhouse_config.native_url.clone();

		let hostname = parsed_url.host_str().unwrap_or("localhost");
		let port = parsed_url.port().unwrap_or(9440).to_string();

		let ca_path = "/usr/local/share/ca-certificates/clickhouse-ca.crt";
		let config = json!({
			"user": clickhouse_config.username,
			"password": clickhouse_config.password.as_ref().map(|x| x.read().clone()),
			"openSSL": if Path::new(&ca_path).exists() {
				json!({
					"client": {
						"caConfig": ca_path
					}
				})
			} else {
				json!(null)
			}
		});

		let mut config_file = tempfile::Builder::new().suffix(".yaml").tempfile()?;
		serde_yaml::to_writer(&mut config_file, &config)?;

		let mut cmd = std::process::Command::new("clickhouse-client");
		cmd.arg("--host")
			.arg(hostname)
			.arg("--port")
			.arg(&port)
			.arg("--user")
			.arg(&clickhouse_config.username)
			.arg("--database")
			.arg(db_name)
			.arg("--config-file")
			.arg(config_file.path());
		if let Some(password) = &clickhouse_config.password {
			cmd.arg("--password").arg(password.read());
		}

		if let Some(query) = query {
			cmd.arg("--multiquery").arg(query);
		}

		cmd.status()?;
	}

	Ok(())
}

pub async fn wf_sqlite_shell(
	config: rivet_config::Config,
	shell_ctx: ShellContext<'_>,
	internal: bool,
) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	let pools = rivet_pools::Pools::new(config.clone()).await?;

	// Combine all queries into one command
	for ShellQuery {
		svc: workflow_id,
		query,
	} in queries
	{
		let workflow_id = Uuid::from_str(workflow_id).context("could not parse input as UUID")?;

		rivet_term::status::warn(
			"WARNING",
			"Database will open in WRITE mode. Modifications made will automatically be committed after the shell closes. This may cause changes made outside of this shell to be overwritten."
		);

		let term = rivet_term::terminal();
		let response = rivet_term::prompt::PromptBuilder::default()
			.message("Are you sure?")
			.build()
			.expect("failed to build prompt")
			.bool(&term)
			.await
			.expect("failed to show prompt");

		if !response {
			return Ok(());
		}

		println!();

		let key = if internal {
			chirp_workflow::db::sqlite_db_name_internal(workflow_id)
		} else {
			chirp_workflow::db::sqlite_db_name_data(workflow_id)
		};
		let key_packed = Arc::new(key.pack_to_vec());

		let db_file = tempfile::NamedTempFile::new()?;
		let db_path = db_file.path();

		read_from_fdb(&pools, &key_packed, &db_path).await?;

		let mut cmd = std::process::Command::new("/root/go/bin/usql");
		cmd.arg(format!("sqlite:{}", db_path.display()));

		if let Some(query) = query {
			cmd.args(["-c", query]);
		}

		cmd.status().context("failed running usql")?;

		rivet_term::status::progress("Evicting database", "");
		write_to_fdb(&pools, &key_packed, &db_path).await?;
		rivet_term::status::success("Evicted", "");
	}

	Ok(())
}

async fn read_from_fdb(
	pools: &rivet_pools::Pools,
	key_packed: &KeyPacked,
	db_path: &Path,
) -> Result<()> {
	let (data, chunks) = pools
		.fdb()?
		.run(|tx, _mc| {
			let key_packed = key_packed.clone();
			async move {
				let compressed_db_data_subspace =
					subspace().subspace(&keys::CompressedDbDataKey::new(key_packed.clone()));

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
					let key = subspace()
						.unpack::<keys::CompressedDbDataChunkKey>(entry.key())
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					// Validate chunk
					if chunk_count != key.chunk {
						return Err(FdbBindingError::CustomError("mismatched chunk".into()));
					}
					chunk_count += 1;

					// Write to buffer
					compressed_data_buf.extend(entry.value());
				}

				// Decompress the data
				let mut decoder = lz4_flex::frame::FrameDecoder::new(&compressed_data_buf[..]);
				decoder
					.read_to_end(&mut buf)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// If there is no compressed data, read from the uncompressed data (backwards compatibility)
				if chunk_count == 0 {
					let db_data_subspace =
						subspace().subspace(&keys::DbDataKey::new(key_packed.clone()));
					let mut data_stream = tx.get_ranges_keyvalues(
						fdb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&db_data_subspace).into()
						},
						SERIALIZABLE,
					);

					while let Some(entry) = data_stream.try_next().await? {
						// Parse key
						let key = subspace()
							.unpack::<keys::DbDataChunkKey>(entry.key())
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						// Validate chunk
						if chunk_count != key.chunk {
							return Err(FdbBindingError::CustomError("mismatched chunk".into()));
						}
						chunk_count += 1;

						// Write to buffer
						buf.extend(entry.value());
					}
				}

				Ok((buf, chunk_count))
			}
		})
		.await?;

	ensure!(chunks > 0, "db not found in fdb");

	tokio::fs::write(db_path, data).await?;

	Ok(())
}

async fn write_to_fdb(
	pools: &rivet_pools::Pools,
	key_packed: &KeyPacked,
	db_path: &Path,
) -> Result<()> {
	let db_url = format!("sqlite://{}", db_path.display());

	let opts = db_url
		.parse::<SqliteConnectOptions>()?
		.create_if_missing(false)
		// Enable foreign key constraint enforcement
		.foreign_keys(true)
		// Enable auto vacuuming and set it to incremental mode for gradual space reclaiming
		.auto_vacuum(SqliteAutoVacuum::Incremental)
		// Set synchronous mode to NORMAL for performance and data safety balance
		.synchronous(SqliteSynchronous::Normal)
		// Increases write performance
		.journal_mode(SqliteJournalMode::Wal)
		// Reduces file system operations
		.locking_mode(SqliteLockingMode::Exclusive);

	let pool_opts = sqlx::sqlite::SqlitePoolOptions::new()
		// The default connection timeout is too high
		.acquire_timeout(Duration::from_secs(60))
		.max_lifetime(Duration::from_secs(15 * 60))
		.max_lifetime_jitter(Duration::from_secs(90))
		// Remove connections after a while in order to reduce load after bursts
		.idle_timeout(Some(Duration::from_secs(10 * 60)))
		// Sqlite doesnt support more than 1 concurrent writer, will get "database is locked"
		.min_connections(1)
		.max_connections(1);

	// Create pool
	let pool = pool_opts.connect_with(opts).await?;

	// Attempt to use an existing connection
	let mut conn = if let Some(conn) = pool.try_acquire() {
		conn
	} else {
		// Create a new connection
		pool.acquire().await?
	};

	// Flush WAL journal
	sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
		.execute(&mut *conn)
		.await?;

	// Stream the database file and compress it
	let mut compressed_data = Vec::new();
	let file = tokio::fs::File::open(db_path).await?;
	let mut reader = tokio::io::BufReader::new(file);
	let mut encoder = lz4_flex::frame::FrameEncoder::new(&mut compressed_data);

	async {
		let mut buffer = [0u8; 16 * 1024]; // 16 KiB
		loop {
			let bytes_read = reader.read(&mut buffer).await?;
			if bytes_read == 0 {
				break;
			}
			encoder.write_all(&buffer[..bytes_read])?;
		}
		encoder.finish()?;

		Result::<_, Error>::Ok(())
	}
	.await?;

	let data = Arc::new(compressed_data);

	// Write to FDB
	pools
		.fdb()?
		.run(|tx, _mc| {
			let key_packed = key_packed.clone();
			let data = data.clone();
			async move {
				// Clear previous data
				let db_data_subspace =
					subspace().subspace(&keys::DbDataKey::new(key_packed.clone()));
				tx.clear_subspace_range(&db_data_subspace);
				let compressed_db_data_subspace =
					subspace().subspace(&keys::CompressedDbDataKey::new(key_packed.clone()));
				tx.clear_subspace_range(&compressed_db_data_subspace);

				// Write chunks
				for (idx, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
					let chunk_key = keys::CompressedDbDataChunkKey {
						db_name_segment: key_packed.clone(),
						chunk: idx,
					};

					tx.set(&subspace().pack(&chunk_key), chunk);
				}

				Ok(())
			}
		})
		.await?;

	Ok(())
}

fn subspace() -> fdb_util::Subspace {
	fdb_util::Subspace::new(&(RIVET, SQLITE))
}
