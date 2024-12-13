use anyhow::*;
use futures_util::stream::{self, StreamExt};
use indoc::formatdoc;
use rivet_config::config::CockroachDbUserRole;
use rivet_pools::prelude::*;
use sqlx::prelude::*;
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	task::block_in_place,
};
use urlencoding::encode;

use crate::{SqlService, SqlServiceKind};

struct MigrateCmd {
	service: SqlService,
	args: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn up(config: rivet_config::Config, services: &[SqlService]) -> Result<()> {
	tracing::info!(sql_services = ?services.len(), "running sql migrations");

	let server_config = config.server.as_ref().context("missing server")?;
	let is_development = server_config.rivet.auth.access_kind
		== rivet_config::config::rivet::AccessKind::Development;

	let crdb = rivet_pools::db::crdb::setup(config.clone())
		.await
		.map_err(|err| anyhow!("{err}"))?;
	let clickhouse =
		rivet_pools::db::clickhouse::setup(config.clone()).map_err(|err| anyhow!("{err}"))?;

	let mut crdb_pre_queries = Vec::new();
	let mut crdb_post_queries = Vec::new();
	let mut clickhouse_pre_queries = Vec::new();
	let clickhouse_post_queries = Vec::<String>::new();

	// Schemas often take too long to apply. These settings will make the
	// migrations apply immediately.
	//
	// https://www.cockroachlabs.com/docs/stable/local-testing.html#use-a-local-single-node-cluster-with-in-memory-storage
	// if is_development {
	// 	crdb_pre_queries
	// 		.push("SET CLUSTER SETTING kv.range_merge.queue_interval = '50ms';".to_string());
	// 	crdb_pre_queries.push("SET CLUSTER SETTING jobs.registry.interval.gc = '30s';".to_string());
	// 	crdb_pre_queries
	// 		.push("SET CLUSTER SETTING jobs.registry.interval.cancel = '180s';".to_string());
	// 	crdb_pre_queries.push("SET CLUSTER SETTING jobs.retention_time = '15s';".to_string());
	// 	crdb_pre_queries.push(
	// 		"SET CLUSTER SETTING sql.stats.automatic_collection.enabled = false;".to_string(),
	// 	);
	// 	crdb_pre_queries
	// 		.push("SET CLUSTER SETTING kv.range_split.by_load_merge_delay = '5s';".to_string());
	// 	crdb_pre_queries
	// 		.push("ALTER RANGE default CONFIGURE ZONE USING \"gc.ttlseconds\" = 600;".to_string());
	// 	crdb_pre_queries.push(
	// 		"ALTER DATABASE system CONFIGURE ZONE USING \"gc.ttlseconds\" = 600;".to_string(),
	// 	);
	// }

	// Run migrations
	for svc in services {
		match &svc.kind {
			SqlServiceKind::CockroachDB => {
				let db_name = &svc.db_name;
				let query = format!(r#"CREATE DATABASE IF NOT EXISTS "{db_name}";"#);
				crdb_pre_queries.push(query);

				// Create users
				for user in server_config.cockroachdb.provision_users.values() {
					let username = &user.username;
					let password = user.password.read();
					let query = match user.role {
						CockroachDbUserRole::Read => {
							formatdoc!(
								r#"
								CREATE USER IF NOT EXISTS {username}
								WITH PASSWORD '{password}';
								GRANT SELECT
								ON {db_name}.*
								TO {username};
								"#
							)
						}
						CockroachDbUserRole::ReadWrite => {
							formatdoc!(
								r#"
								CREATE USER IF NOT EXISTS {username}
								WITH PASSWORD '{password}';
								GRANT SELECT, INSERT, UPDATE, DELETE
								ON {db_name}.*
								TO {username};
								"#
							)
						}
					};
					crdb_post_queries.push(query);
				}
			}
			SqlServiceKind::ClickHouse => {
				let Some(clickhouse_config) = &server_config.clickhouse else {
					tracing::warn!("clickhouse is disabled, skipping {}", svc.db_name);
					continue;
				};

				let db_name = &svc.db_name;

				clickhouse_pre_queries.push(formatdoc!(
					"
					CREATE ROLE IF NOT EXISTS admin;
					"
				));
				clickhouse_pre_queries.push(formatdoc!(
					"
					GRANT CREATE DATABASE ON *.* TO admin;
					"
				));
				clickhouse_pre_queries.push(formatdoc!(
					"
					GRANT
						CREATE TABLE, DROP TABLE, INSERT, SELECT
					ON {db_name}.* TO admin;
					"
				));

				clickhouse_pre_queries.push(formatdoc!(
					"
					CREATE ROLE IF NOT EXISTS write;
					"
				));
				clickhouse_pre_queries.push(formatdoc!(
					"
					GRANT
						INSERT, SELECT
					ON {db_name}.* TO write;
					"
				));
				clickhouse_pre_queries.push(formatdoc!(
					"
					CREATE ROLE IF NOT EXISTS readonly;
					"
				));
				clickhouse_pre_queries.push(formatdoc!(
					"
					GRANT
						SELECT
					ON {db_name}.* TO readonly;
					"
				));

				for user in clickhouse_config.provision_users.values() {
					let username = &user.username;
					let password = user.password.read();

					clickhouse_pre_queries.push(formatdoc!(
						"
						CREATE USER
						IF NOT EXISTS {username}
						IDENTIFIED WITH sha256_password BY '{password}';
						"
					));
					clickhouse_pre_queries.push(formatdoc!(
						"
						GRANT {} TO {username};
						",
						user.role.to_clickhouse_role()
					));
				}

				let query = format!(r#"CREATE DATABASE IF NOT EXISTS "{db_name}";"#);
				clickhouse_pre_queries.push(query);
			}
		}
	}

	// Run pre-migration queries in parallel
	tracing::debug!(crdb = ?crdb_pre_queries.len(), clickhouse = ?clickhouse_pre_queries.len(), "running pre-migrations");
	tokio::try_join!(
		async {
			if !crdb_pre_queries.is_empty() {
				let mut conn = crdb.acquire().await.context("can't acquire crdb")?;

				for query in crdb_pre_queries {
					conn.execute(query.as_str()).await.with_context(|| {
						format!("failed executing crdb pre-migration:\n{query}")
					})?;
				}
			}
			Ok(())
		},
		async {
			if !clickhouse_pre_queries.is_empty() {
				let clickhouse = clickhouse.as_ref().context("missing clickhouse")?;

				for query in clickhouse_pre_queries {
					clickhouse.query(&query).execute().await.with_context(|| {
						format!("failed executing clickhouse pre-migration:\n{query}")
					})?;
				}
			}

			Ok(())
		}
	)?;

	tracing::debug!("running migrations");

	let migrations = services
		.iter()
		// Exclude ClickHouse if needed
		.filter(|svc| clickhouse.is_some() || !matches!(&svc.kind, SqlServiceKind::ClickHouse))
		// Create command
		.map(|svc| MigrateCmd {
			service: svc.clone(),
			args: vec!["up".to_string()],
		})
		.collect::<Vec<_>>();
	run_migrations(config.clone(), &migrations).await?;

	// Run post-migration queries in parallel
	tracing::debug!(crdb = ?crdb_post_queries.len(), clickhouse = ?clickhouse_post_queries.len(), "running post-migrations");
	tokio::try_join!(
		async {
			if !crdb_post_queries.is_empty() {
				for query in crdb_post_queries {
					let mut conn = crdb.acquire().await?;
					conn.execute(query.as_str()).await?;
				}
			}
			Ok(())
		},
		async {
			if !clickhouse_post_queries.is_empty() {
				let clickhouse = clickhouse.as_ref().context("missing clickhouse")?;
				for query in clickhouse_post_queries {
					clickhouse.query(&query).execute().await?;
				}
			}

			Ok(())
		}
	)?;

	wait_for_crdb_schema_migrations(&crdb).await?;

	tracing::debug!("shutting down pools");
	crdb.close().await;

	tracing::debug!("migrated");

	Ok(())
}

pub async fn down(config: rivet_config::Config, service: &SqlService, num: usize) -> Result<()> {
	run_migrations(
		config,
		&[MigrateCmd {
			service: service.clone(),
			args: vec!["down".into(), num.to_string()],
		}],
	)
	.await
}

pub async fn force(config: rivet_config::Config, service: &SqlService, num: usize) -> Result<()> {
	run_migrations(
		config,
		&[MigrateCmd {
			service: service.clone(),
			args: vec!["force".to_string(), num.to_string()],
		}],
	)
	.await
}

pub async fn drop(config: rivet_config::Config, service: &SqlService) -> Result<()> {
	run_migrations(
		config,
		&[MigrateCmd {
			service: service.clone(),
			args: vec!["drop".to_string(), "-f".to_string()],
		}],
	)
	.await
}

async fn run_migrations(config: rivet_config::Config, migration_cmds: &[MigrateCmd]) -> Result<()> {
	let is_dev = config
		.server()
		.map_err(|err| anyhow!("{err}"))?
		.rivet
		.auth
		.access_kind
		== rivet_config::config::AccessKind::Development;
	let migration_parallelism = if is_dev {
		// Speed up migrations when setting up dev clusters since these are usually noops
		16
	} else {
		// Run 1 migration at a time for production environments since these object have costly
		// jobs including building indexes
		1
	};

	let migration_futs = migration_cmds.iter().map(|cmd| {
		let config = config.clone();
		run_migration(config, cmd)
	});

	stream::iter(migration_futs)
		.buffer_unordered(migration_parallelism)
		.collect::<Vec<_>>()
		.await
		// Convert to error
		.into_iter()
		.collect::<Result<Vec<()>, _>>()?;

	Ok(())
}

async fn run_migration(config: rivet_config::Config, cmd: &MigrateCmd) -> Result<()> {
	tracing::debug!(db_name=%cmd.service.db_name, "running db migration");

	// Write migrations to temp path
	let dir = tempfile::tempdir()?;
	block_in_place(|| cmd.service.migrations.extract(dir.path()))?;

	// Run migration
	let migrate_url = migrate_db_url(config, &cmd.service).await?;
	let mut child = tokio::process::Command::new("migrate")
		.arg("-database")
		.arg(migrate_url)
		.arg("-path")
		.arg(dir.path().join("migrations"))
		.args(&cmd.args)
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::piped())
		.spawn()
		.context("failed to run migrate command")?;

	// Log output in real-time
	let stdout = child.stdout.take().context("Failed to capture stdout")?;
	let stderr = child.stderr.take().context("Failed to capture stderr")?;

	tokio::spawn(async move {
		let mut stdout_reader = BufReader::new(stdout).lines();
		while let Some(line) = stdout_reader
			.next_line()
			.await
			.expect("Failed to read stdout")
		{
			tracing::debug!("migrate stdout: {}", line);
		}
	});

	tokio::spawn(async move {
		let mut stderr_reader = BufReader::new(stderr).lines();
		while let Some(line) = stderr_reader
			.next_line()
			.await
			.expect("Failed to read stderr")
		{
			tracing::debug!("migrate stderr: {}", line);
		}
	});

	let status = child.wait().await?;
	if !status.success() {
		bail!("migrate failed: {}", cmd.service.db_name);
	}

	Ok(())
}

/// Returns the URL to use for database migrations.
async fn migrate_db_url(config: rivet_config::Config, service: &SqlService) -> Result<String> {
	let server_config = config.server.as_ref().context("missing server")?;

	match &service.kind {
		SqlServiceKind::CockroachDB => {
			let crdb_url_parsed = server_config.cockroachdb.url.clone();
			let crdb_host = crdb_url_parsed.host_str().context("crdb missing host")?;
			let crdb_port = crdb_url_parsed
				.port_or_known_default()
				.context("crdb missing port")?;

			let auth = if let Some(password) = &server_config.cockroachdb.password {
				format!(
					"{}:{}",
					encode(&server_config.cockroachdb.username),
					encode(password.read())
				)
			} else {
				encode(&server_config.cockroachdb.username).to_string()
			};

			let mut url = url::Url::parse(&format!(
				"cockroach://{auth}@{crdb_host}:{crdb_port}/{}",
				encode(service.db_name)
			))?;

			if let Some(sslmode) = crdb_url_parsed.query_pairs().find(|(k, _)| k == "sslmode") {
				url.query_pairs_mut().append_pair("sslmode", &sslmode.1);
			}

			Ok(url.to_string())
		}
		SqlServiceKind::ClickHouse => {
			let clickhouse_config = server_config
				.clickhouse
				.as_ref()
				.context("missing clickhouse")?;
			let clickhouse_url_parsed = clickhouse_config.native_url.clone();
			let clickhouse_host = clickhouse_url_parsed
				.host_str()
				.context("clickhouse missing host")?;
			let clickhouse_port = clickhouse_url_parsed
				.port_or_known_default()
				.context("clickhouse missing port")?;

			let mut query = format!(
				"database={db}&username={username}&x-multi-statement=true&x-migrations-table-engine=ReplicatedMergeTree&secure={secure}&skip_verify=true",
				db = encode(service.db_name),
				username = encode(&clickhouse_config.username),
				secure = clickhouse_config.secure,
			);
			if let Some(password) = &clickhouse_config.password {
				query += &format!("&password={}", encode(password.read()));
			}

			Ok(format!(
				"clickhouse://{clickhouse_host}:{clickhouse_port}/?&{query}"
			))
		}
	}
}

/// Wait until all pending schema changes have finished applying.
async fn wait_for_crdb_schema_migrations(crdb: &rivet_pools::db::crdb::CrdbPool) -> Result<()> {
	tracing::debug!("waiting for crdb migrations to finish applying");

	loop {
		let mut conn = crdb.acquire().await.context("can't acquire crdb")?;
		let rows: Vec<(String,)> = sqlx::query_as(
            "WITH jobs AS (SHOW JOBS) SELECT job_id FROM jobs WHERE (job_type = 'SCHEMA CHANGE' OR job_type = 'NEW SCHEMA_CHANGE') AND finished IS NULL;"
        )
        .fetch_all(&mut *conn)
        .await
        .context("failed to fetch schema change jobs")?;

		if rows.is_empty() {
			break;
		}

		tracing::info!("waiting for {} schema change jobs to finish", rows.len());
		tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
	}

	Ok(())
}
