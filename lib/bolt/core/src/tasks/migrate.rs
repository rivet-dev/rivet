use std::{
	collections::HashMap,
	fmt,
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::*;
use bolt_config::service::RuntimeKind;
use duct::cmd;
use indoc::formatdoc;
use serde_json::json;
use tokio::{fs, io::AsyncWriteExt, task::block_in_place};

use super::db;
use crate::{
	context::{ProjectContext, ServiceContext},
	utils::{self, db_conn::DatabaseConnections},
};

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

pub async fn create(
	_ctx: &ProjectContext,
	service: &ServiceContext,
	migration_name: &str,
) -> Result<()> {
	let db_ext = match &service.config().runtime {
		RuntimeKind::CRDB { .. } => "sql",
		RuntimeKind::ClickHouse { .. } => "sql",
		x @ _ => bail!("cannot migrate this type of service: {x:?}"),
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

pub async fn check(_ctx: &ProjectContext, services: &[ServiceContext]) -> Result<()> {
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

	// Run migrations
	for svc in services {
		eprintln!();

		match &svc.config().runtime {
			RuntimeKind::CRDB { .. } => {
				let db_name = svc.crdb_db_name();
				rivet_term::status::progress("Migrating Cockroach", &db_name);

				rivet_term::status::progress("Creating database", &db_name);
				let query = format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";");
				db::crdb_shell(db::ShellContext {
					ctx,
					svc,
					conn: &conn,
					query: Some(&query),
				})
				.await?;
			}
			RuntimeKind::ClickHouse { .. } => {
				let db_name = svc.clickhouse_db_name();
				rivet_term::status::progress("Migrating ClickHouse", &db_name);

				rivet_term::status::progress("Creating roles", &db_name);
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
				db::clickhouse_shell(
					db::ShellContext {
						ctx,
						svc,
						conn: &conn,
						query: Some(&query),
					},
					true,
				)
				.await?;

				rivet_term::status::progress("Creating users", &db_name);

				let mut query = String::new();
				for (username, role) in [
					("bolt", ClickhouseRole::Admin),
					("chirp", ClickhouseRole::Write),
					("grafana", ClickhouseRole::Readonly),
				] {
					let password = ctx
						.read_secret(&["clickhouse", "users", username, "password"])
						.await?;

					query.push_str(&formatdoc!(
						"
						CREATE USER
						IF NOT EXISTS {username}
						IDENTIFIED WITH sha256_password BY '{password}';
						GRANT {role} TO {username};
						"
					));
				}

				db::clickhouse_shell(
					db::ShellContext {
						ctx,
						svc,
						conn: &conn,
						query: Some(&query),
					},
					true,
				)
				.await?;

				rivet_term::status::progress("Creating database", &db_name);
				let query = format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";");
				db::clickhouse_shell(
					db::ShellContext {
						ctx,
						svc,
						conn: &conn,
						query: Some(&query),
					},
					true,
				)
				.await?;
			}
			x @ _ => bail!("cannot migrate this type of service: {x:?}"),
		}

		rivet_term::status::progress("Running migrations", "");
		let database_url = conn.migrate_db_url(svc).await?;
		migration(ctx, svc, &["up"], database_url).await?;

		rivet_term::status::success("Migrated", "");
	}

	Ok(())
}

pub async fn down(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(
		ctx,
		service,
		&["down", num.to_string().as_str()],
		database_url,
	)
	.await
}

pub async fn force(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(
		ctx,
		service,
		&["force", num.to_string().as_str()],
		database_url,
	)
	.await
}

pub async fn drop(ctx: &ProjectContext, service: &ServiceContext) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	migration(ctx, service, &["drop", "-f"], database_url).await
}

pub async fn migration(
	ctx: &ProjectContext,
	svc: &ServiceContext,
	migration_cmd: &[&str],
	database_url: String,
) -> Result<()> {
	upload_migrations(ctx, svc).await?;

	let cmd = formatdoc!(
		"
		sleep 2 &&
		apk update -q &&
		apk add -q tzdata &&
		migrate \
		-database \"{database_url}\" \
		-path /local/migrations \
		{}
		",
		migration_cmd.join(" ")
	);
	// let cmd = "sleep 2000";
	let overrides = json!({
		"apiVersion": "v1",
		"metadata": {
			"namespace": "bolt",
		},
		"spec": {
			"containers": [
				{
					"name": "migrate",
					"image": "migrate/migrate",
					"command": ["sh", "-c"],
					"args": [cmd],
					// // See https://github.com/golang-migrate/migrate/issues/494
					// "env": [{
					// 	"name": "TZ",
					// 	"value": "UTC"
					// }],
					"volumeMounts": [
						{
							"name": "migrations",
							"mountPath": "/local/migrations",
							"readOnly": true
						},
						{
							"name": "crdb-ca",
							"mountPath": "/local/crdb-ca.crt",
							"subPath": "crdb-ca.crt"
						}
					]
				}
			],
			"volumes": [
				{
					"name": "migrations",
					"configMap": {
						"name": format!("{}-migrations", svc.name()),
						"defaultMode": 420
					}
				},
				{
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
				}
			]
		}
	});

	block_in_place(|| {
		cmd!(
			"kubectl",
			"run",
			"-itq",
			"--rm",
			"--restart=Never",
			"--image=migrate/migrate",
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
	cmd.args(&["apply", "-f", "-"]);
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
