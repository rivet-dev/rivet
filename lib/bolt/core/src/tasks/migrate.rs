use anyhow::*;
use bolt_config::service::RuntimeKind;
use duct::cmd;
use std::time::Duration;
use tokio::task::block_in_place;

use crate::{
	context::{ProjectContext, ServiceContext},
	utils::{self, db_conn::DatabaseConnection},
};

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
	let conn = DatabaseConnection::create(ctx, services).await?;

	// Run migrations
	for svc in services {
		let database_url = conn.migrate_db_url(svc).await?;

		eprintln!();

		match &svc.config().runtime {
			RuntimeKind::CRDB { .. } => {
				let db_name = svc.crdb_db_name();
				rivet_term::status::progress("Migrating Cockroach", &db_name);

				let host = conn.cockroach_host.as_ref().unwrap();
				let (hostname, port) = host.split_once(":").unwrap();
				let username = ctx.read_secret(&["crdb", "username"]).await?;
				let password = ctx.read_secret(&["crdb", "password"]).await?;
				let conn = format!(
					"postgres://{}:{}@{}/{}?sslmode=verify-ca&sslrootcert=/tmp/crdb-ca.crt",
					username, password, host, db_name
				);

				rivet_term::status::progress("Creating database", &db_name);
				block_in_place(|| {
					cmd!(
						"psql",
						conn,
						"-c",
						format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";"),
					)
					// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
					.env("PGCLIENTENCODING", "utf-8")
					.run()
				})?;
			}
			RuntimeKind::ClickHouse { .. } => {
				let db_name = svc.clickhouse_db_name();
				rivet_term::status::progress("Migrating ClickHouse", &db_name);

				let clickhouse_user = "bolt";
				let clickhouse_password = ctx
					.read_secret(&["clickhouse", "users", "bolt", "password"])
					.await?;
				let host = conn.clickhouse_host.as_ref().unwrap();
				let (hostname, port) = host.split_once(":").unwrap();

				rivet_term::status::progress("Creating database", &db_name);
				block_in_place(|| {
					cmd!(
						"clickhouse",
						"client",
						"--config-file",
						"/tmp/clickhouse-config.yml",
						"--host",
						hostname,
						"--port",
						port,
						"--user",
						clickhouse_user,
						"--password",
						clickhouse_password,
						"--query",
						format!("CREATE DATABASE IF NOT EXISTS \"{db_name}\";")
					)
					.run()
				})?;
			}
			x @ _ => bail!("cannot migrate this type of service: {x:?}"),
		}

		rivet_term::status::progress("Running migrations", "");
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

		rivet_term::status::success("Migrated", "");
	}

	Ok(())
}

pub async fn down(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnection::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	block_in_place(|| {
		cmd!(
			"migrate",
			"-database",
			database_url,
			"-path",
			service.migrations_path(),
			"down",
			num.to_string(),
		)
		.run()
	})?;

	Ok(())
}

pub async fn force(ctx: &ProjectContext, service: &ServiceContext, num: usize) -> Result<()> {
	let conn = DatabaseConnection::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	block_in_place(|| {
		cmd!(
			"migrate",
			"-database",
			database_url,
			"-path",
			service.migrations_path(),
			"force",
			num.to_string(),
		)
		.run()
	})?;

	Ok(())
}

pub async fn drop(ctx: &ProjectContext, service: &ServiceContext) -> Result<()> {
	let conn = DatabaseConnection::create(ctx, &[service.clone()]).await?;
	let database_url = conn.migrate_db_url(service).await?;

	block_in_place(|| {
		cmd!(
			"migrate",
			"-database",
			database_url,
			"-path",
			service.migrations_path(),
			"drop",
		)
		.run()
	})?;

	Ok(())
}
