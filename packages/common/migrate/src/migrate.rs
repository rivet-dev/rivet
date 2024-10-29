use anyhow::*;
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

pub async fn up(config: rivet_config::Config, services: &[SqlService]) -> Result<()> {
	let server_config = config.server.as_ref().context("missing server")?;

	let crdb = rivet_pools::db::crdb::setup(config.clone())
		.await
		.map_err(|err| anyhow!("{err}"))?;
	let clickhouse =
		rivet_pools::db::clickhouse::setup(config.clone()).map_err(|err| anyhow!("{err}"))?;

	let mut crdb_pre_queries = Vec::new();
	let mut crdb_post_queries = Vec::new();
	let mut clickhouse_pre_queries = Vec::new();
	let clickhouse_post_queries = Vec::<String>::new();

	// Run migrations
	for svc in services {
		match &svc.kind {
			SqlServiceKind::CockroachDB => {
				let db_name = &svc.db_name;
				let query = format!(r#"CREATE DATABASE IF NOT EXISTS "{db_name}";"#);
				crdb_pre_queries.push(query);

				// Create users
				for (_, user) in &server_config.cockroachdb.provision_users {
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

				for (_, user) in &clickhouse_config.provision_users {
					let username = &user.username;
					let password = &*user.password.read();

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
	tracing::info!(crdb = ?crdb_pre_queries.len(), clickhouse = ?clickhouse_pre_queries.len(), "running pre-migrations");
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

	tracing::info!("running migrations");

	let migrations = services
		.iter()
		// TODO: Remove this. Disable ClickHouse since it's not working at the moment.
		.filter(|svc| !matches!(&svc.kind, SqlServiceKind::ClickHouse))
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
	tracing::info!(crdb = ?crdb_post_queries.len(), clickhouse = ?clickhouse_post_queries.len(), "running post-migrations");
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

	tracing::info!("shutting down pools");
	crdb.close().await;

	tracing::info!("migrated");

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
	for cmd in migration_cmds {
		tracing::info!(db_name=%cmd.service.db_name, "running db migration");

		// Write migrations to temp path
		let dir = tempfile::tempdir()?;
		block_in_place(|| cmd.service.migrations.extract(dir.path()))?;

		// Run migration
		let migrate_url = migrate_db_url(config.clone(), &cmd.service).await?;
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
		let stdout = child.stdout.take().expect("Failed to capture stdout");
		let stderr = child.stderr.take().expect("Failed to capture stderr");

		tokio::spawn(async move {
			let mut stdout_reader = BufReader::new(stdout).lines();
			while let Some(line) = stdout_reader
				.next_line()
				.await
				.expect("Failed to read stdout")
			{
				tracing::info!("migrate stdout: {}", line);
			}
		});

		tokio::spawn(async move {
			let mut stderr_reader = BufReader::new(stderr).lines();
			while let Some(line) = stderr_reader
				.next_line()
				.await
				.expect("Failed to read stderr")
			{
				tracing::warn!("migrate stderr: {}", line);
			}
		});

		let status = child.wait().await?;
		if !status.success() {
			tracing::error!("migrate failed: {}", cmd.service.db_name);
			std::future::pending::<()>().await;
			unreachable!();
		}
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
				encode(&service.db_name)
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
			let clickhouse_url_parsed = clickhouse_config.url.clone();
			let clickhouse_host = clickhouse_url_parsed
				.host_str()
				.context("clickhouse missing host")?;
			let clickhouse_port = clickhouse_url_parsed
				.port_or_known_default()
				.context("clickhouse missing port")?;

			let mut query = format!(
				"database={db}&username={username}&x-multi-statement=true&x-migrations-table-engine=ReplicatedMergeTree&secure=true&skip_verify=true",
				db = encode(&service.db_name),
				username = encode(&clickhouse_config.username),
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
