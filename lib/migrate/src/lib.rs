use std::fmt;

use anyhow::*;
use indoc::formatdoc;
use rivet_pools::prelude::*;
use sqlx::prelude::*;
use tokio::task::block_in_place;
use urlencoding::encode;

use registry::{SqlService, SqlServiceKind};

pub mod registry;

enum ClickhouseRole {
	Admin,
	Write,
	Readonly,
}

impl fmt::Display for ClickhouseRole {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ClickhouseRole::Admin => write!(f, "admin"),
			ClickhouseRole::Write => write!(f, "write"),
			ClickhouseRole::Readonly => write!(f, "readonly"),
		}
	}
}

struct MigrateCmd {
	service: &'static SqlService,
	args: Vec<String>,
}

pub async fn up(services: &[&'static SqlService]) -> Result<()> {
	let pools = rivet_pools::from_env("rivet-migrate").await?;

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
				let username =
					rivet_util::env::read_secret(&["crdb", "user", "grafana", "username"]).await?;
				let password =
					rivet_util::env::read_secret(&["crdb", "user", "grafana", "password"]).await?;
				let query = formatdoc!(
					r#"
					CREATE USER IF NOT EXISTS {username}
					WITH PASSWORD '{password}';
					GRANT SELECT
					ON {db_name}.*
					TO {username};
					"#
				);
				crdb_post_queries.push(query);
			}
			SqlServiceKind::ClickHouse => {
				if !pools.clickhouse_enabled() {
					tracing::warn!("clickhouse is disabled, skipping {}", svc.db_name);
					continue;
				}

				let db_name = &svc.db_name;

				let query = formatdoc!(
					"
					CREATE ROLE IF NOT EXISTS admin;
					GRANT CREATE DATABASE ON *.* TO admin;
					GRANT
						CREATE TABLE, DROP TABLE, INSERT, SELECT
					ON {db_name}.* TO admin;

					CREATE ROLE IF NOT EXISTS write;
					GRANT
						INSERT, SELECT
					ON {db_name}.* TO write;

					CREATE ROLE IF NOT EXISTS readonly;
					GRANT
						SELECT
					ON {db_name}.* TO readonly;
					"
				);
				clickhouse_pre_queries.push(query);

				for (username, role) in [
					("bolt", ClickhouseRole::Admin),
					("chirp", ClickhouseRole::Write),
					("grafana", ClickhouseRole::Readonly),
					("vector", ClickhouseRole::Write),
				] {
					let password = rivet_util::env::read_secret(&[
						"clickhouse",
						"users",
						username,
						"password",
					])
					.await?;

					let query = formatdoc!(
						"
						CREATE USER
						IF NOT EXISTS {username}
						IDENTIFIED WITH sha256_password BY '{password}';
						GRANT {role} TO {username};
						"
					);
					clickhouse_pre_queries.push(query);
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
				let crdb = pools.crdb()?;
				for query in crdb_pre_queries {
					let mut conn = crdb.acquire().await?;
					conn.execute(query.as_str()).await?;
				}
			}
			Ok(())
		},
		async {
			if !clickhouse_pre_queries.is_empty() {
				let clickhouse = pools
					.clickhouse()
					.map_err(|err| anyhow!("failed to acquire clickhouse: {err}"))?;
				for query in clickhouse_pre_queries {
					clickhouse.query(&query).execute().await?;
				}
			}

			Ok(())
		}
	)?;

	tracing::info!("running migrations");

	let migrations = services
		.iter()
		// Exclude ClickHouse if needed
		.filter(|svc| {
			pools.clickhouse_enabled() || !matches!(&svc.kind, SqlServiceKind::ClickHouse)
		})
		// Create command
		.map(|svc| MigrateCmd {
			service: svc,
			args: vec!["up".to_string()],
		})
		.collect::<Vec<_>>();
	run_migrations(&migrations).await?;

	// Run post-migration queries in parallel
	tracing::info!(crdb = ?crdb_post_queries.len(), clickhouse = ?clickhouse_post_queries.len(), "running post-migrations");
	tokio::try_join!(
		async {
			if !crdb_post_queries.is_empty() {
				let crdb = pools.crdb()?;
				for query in crdb_post_queries {
					let mut conn = crdb.acquire().await?;
					conn.execute(query.as_str()).await?;
				}
			}
			Ok(())
		},
		async {
			if !clickhouse_post_queries.is_empty() {
				let clickhouse = pools
					.clickhouse()
					.map_err(|err| anyhow!("failed to acquire clickhouse: {err}"))?;
				for query in clickhouse_post_queries {
					clickhouse.query(&query).execute().await?;
				}
			}

			Ok(())
		}
	)?;

	tracing::info!("migrated");

	Ok(())
}

pub async fn down(service: &'static SqlService, num: usize) -> Result<()> {
	run_migrations(&[MigrateCmd {
		service,
		args: vec!["down".into(), num.to_string()],
	}])
	.await
}

pub async fn force(service: &'static SqlService, num: usize) -> Result<()> {
	run_migrations(&[MigrateCmd {
		service,
		args: vec!["force".to_string(), num.to_string()],
	}])
	.await
}

pub async fn drop(service: &'static SqlService) -> Result<()> {
	run_migrations(&[MigrateCmd {
		service,
		args: vec!["drop".to_string(), "-f".to_string()],
	}])
	.await
}

async fn run_migrations(migration_cmds: &[MigrateCmd]) -> Result<()> {
	for cmd in migration_cmds {
		// Write migrations to temp path
		let dir = tempfile::tempdir()?;
		block_in_place(|| cmd.service.migrations.extract(dir.path()))?;

		// Run migration
		let migrate_url = migrate_db_url(cmd.service).await?;
		let status = tokio::process::Command::new("migrate")
			.arg("-database")
			.arg(migrate_url)
			.arg("-path")
			.arg(dir.path())
			.args(&cmd.args)
			.status()
			.await?;
		ensure!(status.success(), "migrate failed: {}", cmd.service.db_name);
	}

	Ok(())
}

/// Returns the URL to use for database migrations.
async fn migrate_db_url(service: &SqlService) -> Result<String> {
	match &service.kind {
		SqlServiceKind::CockroachDB => {
			let crdb_url = rivet_pools::crdb_url_from_env()?.context("missing crdb_url")?;
			let crdb_url_parsed = url::Url::parse(&crdb_url)?;
			let crdb_host = crdb_url_parsed.host_str().context("crdb missing host")?;

			let username = rivet_util::env::read_secret(&["crdb", "username"]).await?;
			let password = rivet_util::env::read_secret(&["crdb", "password"]).await?;

			Ok(format!(
				"cockroach://{}:{}@{}/{}?sslmode=verify-ca&sslrootcert=/local/crdb-ca.crt",
				encode(&username),
				encode(&password),
				crdb_host,
				encode(&service.db_name),
			))
		}
		SqlServiceKind::ClickHouse => {
			let clickhouse_url =
				rivet_pools::clickhouse_url_from_env()?.context("missing clickhouse_url")?;
			let clickhouse_url_parsed = url::Url::parse(&clickhouse_url)?;
			let clickhouse_host = clickhouse_url_parsed
				.host_str()
				.context("clickhouse missing host")?;

			let clickhouse_user = "bolt";
			let clickhouse_password =
				rivet_util::env::read_secret(&["clickhouse", "users", "bolt", "password"]).await?;

			let query_other = "&x-multi-statement=true&x-migrations-table-engine=ReplicatedMergeTree&secure=true&skip_verify=true".to_string();

			Ok(format!(
				"clickhouse://{}/?database={}&username={}&password={}{}",
				clickhouse_host,
				encode(&service.db_name),
				encode(&clickhouse_user),
				encode(&clickhouse_password),
				query_other,
			))
		}
	}
}
