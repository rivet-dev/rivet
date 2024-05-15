use std::{collections::HashMap, fmt, time::Duration};

use anyhow::*;
use bolt_config::service::RuntimeKind;
use duct::cmd;
use futures_util::{StreamExt, TryStreamExt};
use indoc::formatdoc;
use serde_json::json;
use tokio::{fs, io::AsyncWriteExt, task::block_in_place};

use super::db;
use crate::{
	context::{ProjectContext, ServiceContext},
	utils::{self, db_conn::DatabaseConnections},
};

const MIGRATE_IMAGE: &str = "ghcr.io/rivet-gg/golang-migrate:ea4c84e";

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
	service: ServiceContext,
	database_url: String,
	cmd: String,
}

pub async fn create(
	_ctx: &ProjectContext,
	service: &ServiceContext,
	migration_name: &str,
) -> Result<()> {
	let db_ext = match &service.config().runtime {
		RuntimeKind::CRDB { .. } => "sql",
		RuntimeKind::ClickHouse { .. } => "sql",
		x => bail!("cannot migrate this type of service: {x:?}"),
	};

	block_in_place(|| {
		cmd!(
			"migrate",
			"create",
			"-ext",
			db_ext,
			"-dir",
			service.migrations_path(),
			migration_name,
		)
		.run()
	})?;

	Ok(())
}

pub async fn check_all(ctx: &ProjectContext) -> Result<()> {
	let services = ctx.services_with_migrations().await;
	check(ctx, &services[..]).await
}

pub async fn check(ctx: &ProjectContext, services: &[ServiceContext]) -> Result<()> {
	// Spawn Cockroach test container
	let crdb_port = utils::pick_port();
	let crdb_container_id = if services
		.iter()
		.any(|x| matches!(x.config().runtime, RuntimeKind::CRDB { .. }))
	{
		let image = "cockroachdb/cockroach:v22.2.0";
		rivet_term::status::progress("Creating container", image);
		let container_id_bytes = block_in_place(|| {
			cmd!(
				"docker",
				"run",
				"-d",
				"--rm",
				"-p",
				&format!("{crdb_port}:26257"),
				image,
				"start-single-node",
				"--insecure",
			)
			.stdout_capture()
			.run()
		})?
		.stdout;
		let container_id = String::from_utf8(container_id_bytes)?.trim().to_string();

		// Wait for the service to boot
		rivet_term::status::progress("Waiting for database to start", "");
		loop {
			let test_cmd = block_in_place(|| {
				cmd!(
					"psql",
					"-h",
					"127.0.0.1",
					"-p",
					crdb_port.to_string(),
					"-U",
					"root",
					"postgres",
					"-c",
					"SELECT 1;"
				)
				.stdout_null()
				.stderr_null()
				.run()
			});
			if test_cmd.is_ok() {
				break;
			}

			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		Some(container_id)
	} else {
		None
	};

	// Spawn ClickHouse test container
	let clickhouse_port = utils::pick_port();
	let clickhouse_container_id = if services
		.iter()
		.any(|x| matches!(x.config().runtime, RuntimeKind::ClickHouse { .. }))
	{
		let image = "clickhouse/clickhouse-server:22.12.3.5-alpine";
		rivet_term::status::progress("Creating container", image);
		let container_id_bytes = block_in_place(|| {
			cmd!(
				"docker",
				"run",
				"-d",
				"--rm",
				"-p",
				&format!("{clickhouse_port}:9000"),
				image,
			)
			.stdout_capture()
			.run()
		})?
		.stdout;
		let container_id = String::from_utf8(container_id_bytes)?.trim().to_string();

		// Wait for the service to boot
		rivet_term::status::progress("Waiting for database to start", "");
		loop {
			let test_cmd = block_in_place(|| {
				cmd!(
					"clickhouse",
					"client",
					"-q",
					"--port",
					clickhouse_port.to_string(),
					"SELECT 1;"
				)
				.stdout_null()
				.stderr_null()
				.run()
			});
			if test_cmd.is_ok() {
				break;
			}

			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		Some(container_id)
	} else {
		None
	};

	// Run migrations against test containers
	for svc in services {
		eprintln!();
		rivet_term::status::progress("Checking", svc.name());

		let database_url = match &svc.config().runtime {
			RuntimeKind::CRDB { .. } => {
				// Build URL
				let db_name = svc.crdb_db_name();
				let database_url =
					format!("cockroach://root@127.0.0.1:{crdb_port}/{db_name}?sslmode=disable",);

				// Create database
				block_in_place(|| {
					cmd!(
						"psql",
						"-h",
						"127.0.0.1",
						"-p",
						crdb_port.to_string(),
						"-U",
						"root",
						"postgres",
						"-c",
						format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";"),
					)
					// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
					.env("PGCLIENTENCODING", "utf-8")
					.run()
				})?;

				database_url
			}
			RuntimeKind::ClickHouse { .. } => {
				if ctx.ns().clickhouse.is_none() {
					rivet_term::status::warn(
						"Warning",
						format!("Clickhouse is disabled. Skipping {}", svc.name()),
					);
					continue;
				}

				// Build URL
				let db_name = svc.clickhouse_db_name();
				let database_url =
					format!("clickhouse://127.0.0.1:{clickhouse_port}/?database={db_name}&x-multi-statement=true");

				// Create database
				block_in_place(|| {
					cmd!(
						"clickhouse",
						"client",
						"--port",
						clickhouse_port.to_string(),
						"--query",
						format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";")
					)
					.run()
				})?;

				database_url
			}
			x @ _ => bail!("cannot migrate this type of service: {x:?}"),
		};

		block_in_place(|| {
			cmd!(
				"migrate",
				"-database",
				database_url,
				"-path",
				svc.migrations_path(),
				"up",
			)
			.run()
		})?;

		rivet_term::status::success("Migrations valid", "");
	}

	// Kill containers
	println!();
	if let Some(id) = crdb_container_id {
		rivet_term::status::progress("Killing Cockroach container", "");
		block_in_place(|| cmd!("docker", "stop", "-t", "0", id).run())?;
	}
	if let Some(id) = clickhouse_container_id {
		rivet_term::status::progress("Killing ClickHouse container", "");
		block_in_place(|| cmd!("docker", "stop", "-t", "0", id).run())?;
	}

	Ok(())
}

pub async fn up_all(ctx: &ProjectContext) -> Result<()> {
	let services = ctx.services_with_migrations().await;
	up(ctx, &services[..]).await
}

pub async fn up(ctx: &ProjectContext, services: &[ServiceContext]) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, services).await?;
	let mut crdb_pre_queries = Vec::new();
	let mut crdb_post_queries = Vec::new();
	let mut clickhouse_pre_queries = Vec::new();
	let clickhouse_post_queries = Vec::new();

	// Run migrations
	for svc in services {
		match &svc.config().runtime {
			RuntimeKind::CRDB { .. } => {
				let db_name = svc.crdb_db_name();
				let query = format!(r#"CREATE DATABASE IF NOT EXISTS "{db_name}";"#);

				crdb_pre_queries.push(db::ShellQuery {
					svc: svc.clone(),
					query: Some(query),
				});

				// Create users
				let username = ctx
					.read_secret(&["crdb", "user", "grafana", "username"])
					.await?;
				let password = ctx
					.read_secret(&["crdb", "user", "grafana", "password"])
					.await?;
				let query = formatdoc!(
					r#"
					CREATE USER IF NOT EXISTS {username}
					WITH PASSWORD '{password}';
					GRANT SELECT
					ON {db_name}.*
					TO {username};
					"#
				);
				crdb_post_queries.push(db::ShellQuery {
					svc: svc.clone(),
					query: Some(query),
				});
			}
			RuntimeKind::ClickHouse { .. } => {
				if ctx.ns().clickhouse.is_none() {
					rivet_term::status::warn(
						"Warning",
						format!("Clickhouse is disabled. Skipping {}", svc.name()),
					);
					continue;
				}

				let db_name = svc.clickhouse_db_name();

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
				clickhouse_pre_queries.push(db::ShellQuery {
					svc: svc.clone(),
					query: Some(query),
				});

				for (username, role) in [
					("bolt", ClickhouseRole::Admin),
					("chirp", ClickhouseRole::Write),
					("grafana", ClickhouseRole::Readonly),
					("vector", ClickhouseRole::Write),
				] {
					let password = ctx
						.read_secret(&["clickhouse", "users", username, "password"])
						.await?;

					let query = formatdoc!(
						"
						CREATE USER
						IF NOT EXISTS {username}
						IDENTIFIED WITH sha256_password BY '{password}';
						GRANT {role} TO {username};
						"
					);
					clickhouse_pre_queries.push(db::ShellQuery {
						svc: svc.clone(),
						query: Some(query),
					});
				}

				let query = format!(r#"CREATE DATABASE IF NOT EXISTS "{db_name}";"#);
				clickhouse_pre_queries.push(db::ShellQuery {
					svc: svc.clone(),
					query: Some(query),
				});
			}
			x => bail!("cannot migrate this type of service: {x:?}"),
		}
	}

	// Run pre-migration queries in parallel
	rivet_term::status::progress("Running pre-migrations", "");
	tokio::try_join!(
		async {
			if !crdb_pre_queries.is_empty() {
				db::crdb_shell(db::ShellContext {
					ctx,
					conn: &conn,
					queries: &crdb_pre_queries,
					log_type: db::LogType::Migration,
				})
				.await?;
			}
			Ok(())
		},
		async {
			if !clickhouse_pre_queries.is_empty() {
				db::clickhouse_shell(
					db::ShellContext {
						ctx,
						conn: &conn,
						queries: &clickhouse_pre_queries,
						log_type: db::LogType::Migration,
					},
					true,
				)
				.await?;
			}

			Ok(())
		}
	)?;

	eprintln!();
	rivet_term::status::progress("Running migrations", "");

	let filtered_services = services.iter().filter(|svc| {
		ctx.ns().clickhouse.is_some()
			|| !matches!(&svc.config().runtime, RuntimeKind::ClickHouse { .. })
	});

	let migrations = futures_util::stream::iter(filtered_services)
		.map(|svc| {
			let conn = conn.clone();

			async move {
				let database_url = conn.migrate_db_url(svc).await?;

				Ok(MigrateCmd {
					service: svc.clone(),
					database_url,
					cmd: "up".to_string(),
				})
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;
	migration(ctx, &migrations).await?;

	// Run post-migration queries in parallel
	eprintln!();
	rivet_term::status::progress("Running post-migrations", "");
	tokio::try_join!(
		async {
			if !crdb_post_queries.is_empty() {
				db::crdb_shell(db::ShellContext {
					ctx,
					conn: &conn,
					queries: &crdb_post_queries,
					log_type: db::LogType::Migration,
				})
				.await?;
			}
			Ok(())
		},
		async {
			if !clickhouse_post_queries.is_empty() {
				db::clickhouse_shell(
					db::ShellContext {
						ctx,
						conn: &conn,
						queries: &clickhouse_post_queries,
						log_type: db::LogType::Migration,
					},
					true,
				)
				.await?;
			}

			Ok(())
		}
	)?;

	rivet_term::status::success("Migrated", "");

	Ok(())
}

pub async fn down(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(
		ctx,
		&[MigrateCmd {
			service: service.clone(),
			database_url,
			cmd: format!("down {}", num.to_string().as_str()),
		}],
	)
	.await
}

pub async fn force(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(
		ctx,
		&[MigrateCmd {
			service: service.clone(),
			database_url,
			cmd: format!("force {}", num.to_string().as_str()),
		}],
	)
	.await
}

pub async fn drop(ctx: &ProjectContext, service: &ServiceContext) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(
		ctx,
		&[MigrateCmd {
			service: service.clone(),
			database_url,
			cmd: "drop -f".to_string(),
		}],
	)
	.await
}

async fn migration(ctx: &ProjectContext, migration_cmds: &[MigrateCmd]) -> Result<()> {
	// Combine all commands into one
	let migration_cmd = migration_cmds
		.iter()
		.map(|cmd| {
			let name = cmd.service.name();

			format!(
				"echo Migrating {name} && migrate -database \"{}\" -path /local/migrations/{name} {}",
				cmd.database_url, cmd.cmd
			)
		})
		.collect::<Vec<_>>()
		.join(" && ");
	let mut mounts = vec![json!({
		"name": "crdb-ca",
		"mountPath": "/local/crdb-ca.crt",
		"subPath": "crdb-ca.crt"
	})];
	let mut volumes = vec![json!({
		"name": "crdb-ca",
		"configMap": {
			"name": "crdb-ca",
			"defaultMode": 420,
			"items": [
				{
					"key": "ca.crt",
					"path": "crdb-ca.crt"
				}
			]
		}
	})];

	// Upload all migration files as config maps
	futures_util::stream::iter(
		migration_cmds
			.iter()
			.map(|cmd| upload_migrations(ctx, &cmd.service)),
	)
	.buffer_unordered(8)
	.try_collect::<Vec<_>>()
	.await?;

	// Add volumes
	for cmd in migration_cmds {
		let svc = &cmd.service;
		let label = format!("{}-migrations", svc.name());

		mounts.push(json!({
			"name": label,
			"mountPath": format!("/local/migrations/{}", svc.name()),
			"readOnly": true
		}));
		volumes.push(json!({
			"name": label,
			"configMap": {
				"name": label,
				"defaultMode": 420
			}
		}));
	}

	let overrides = json!({
		"apiVersion": "v1",
		"metadata": {
			"namespace": "bolt",
		},
		"spec": {
			"containers": [
				{
					"name": "migrate",
					"image": MIGRATE_IMAGE,
					"command": ["sh", "-c", migration_cmd],
					// // See https://github.com/golang-migrate/migrate/issues/494
					// "env": [{
					// 	"name": "TZ",
					// 	"value": "UTC"
					// }],
					"volumeMounts": mounts
				}
			],
			"volumes": volumes
		}
	});

	block_in_place(|| {
		cmd!(
			"kubectl",
			"run",
			"-itq",
			"--rm",
			"--restart=Never",
			format!("--image={MIGRATE_IMAGE}"),
			"--namespace",
			"bolt",
			format!("--overrides={overrides}"),
			db::shell_name("migrate"),
		)
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.run()
	})?;

	Ok(())
}

async fn upload_migrations(ctx: &ProjectContext, svc: &ServiceContext) -> Result<()> {
	let mut files = HashMap::new();

	// Read all files in migrations directory
	let mut dir = fs::read_dir(svc.migrations_path()).await?;
	while let Some(entry) = dir.next_entry().await? {
		let meta = entry.metadata().await?;
		if !meta.is_file() {
			bail!("subfolder not allowed in migrations folder");
		}

		let file_name = entry
			.path()
			.file_name()
			.context("path.file_name")?
			.to_str()
			.context("as_str")?
			.to_string();
		let content = fs::read_to_string(entry.path()).await?;

		files.insert(file_name, content);
	}

	// Create config map for all files
	let spec = serde_json::to_vec(&json!({
		"kind": "ConfigMap",
		"apiVersion": "v1",
		"metadata": {
			"name": format!("{}-migrations", svc.name()),
			"namespace": "bolt"
		},
		"data": files
	}))?;

	let mut cmd = tokio::process::Command::new("kubectl");
	cmd.args(["apply", "-f", "-"]);
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());
	cmd.stdin(std::process::Stdio::piped());
	cmd.stdout(std::process::Stdio::null());
	let mut child = cmd.spawn()?;

	{
		let mut stdin = child.stdin.take().context("missing stdin")?;
		stdin.write_all(&spec).await?;
	}

	let status = child.wait().await?;
	ensure!(status.success(), "kubectl apply failed");

	Ok(())
}
