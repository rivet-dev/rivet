use anyhow::*;
use duct::cmd;
use tokio::task::block_in_place;

use crate::{
	config::service::RuntimeKind,
	context::{ProjectContext, ServiceContext},
	utils::db_conn::DatabaseConnection,
};

pub async fn shell(ctx: &ProjectContext, svc: &ServiceContext, query: Option<&str>) -> Result<()> {
	let conn = DatabaseConnection::create(ctx, &[svc.clone()]).await?;

	match &svc.config().runtime {
		RuntimeKind::Redis { .. } => {
			let db_name = svc.redis_db_name();
			let host = conn.redis_hosts.get(&svc.name()).unwrap();
			let (hostname, port) = host.split_once(":").unwrap();
			let username = ctx.read_secret(&["redis", &db_name, "username"]).await?;
			let password = ctx
				.read_secret_opt(&["redis", &db_name, "password"])
				.await?;

			rivet_term::status::progress("Connecting to Redis", &db_name);

			if let Some(_) = query {
				todo!("cannot pass query at the moment")
			} else {
				if let Some(password) = password {
					block_in_place(|| {
						cmd!(
							"redis-cli",
							"-h",
							hostname,
							"-p",
							port,
							"--user",
							username,
							"--tls",
							"--cacert",
							format!("/tmp/{}-ca.crt", svc.name()),
						)
						.env("REDISCLI_AUTH", password)
						.run()
					})?;
				} else {
					block_in_place(|| {
						cmd!(
							"redis-cli",
							"-h",
							hostname,
							"-p",
							port,
							"--user",
							username,
							"--tls",
							"--cacert",
							format!("/tmp/{}-ca.crt", svc.name()),
						)
						.run()
					})?;
				}
			}
		}
		RuntimeKind::CRDB { .. } => {
			let db_name = svc.crdb_db_name();
			let host = conn.cockroach_host.as_ref().unwrap();
			let username = ctx.read_secret(&["crdb", "username"]).await?;
			let password = ctx.read_secret(&["crdb", "password"]).await?;
			let conn = format!(
				"postgres://{}:{}@{}/{}?sslmode=verify-ca&sslrootcert=/tmp/crdb-ca.crt",
				username, password, host, db_name
			);

			rivet_term::status::progress("Connecting to Cockroach", &db_name);
			if let Some(query) = query {
				block_in_place(|| {
					cmd!("psql", conn, "-c", query)
						// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
						.env("PGCLIENTENCODING", "utf-8")
						.run()
				})?;
			} else {
				block_in_place(|| {
					cmd!("psql", conn)
						// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
						.env("PGCLIENTENCODING", "utf-8")
						.run()
				})?;
			}
		}
		RuntimeKind::ClickHouse { .. } => {
			let db_name = svc.clickhouse_db_name();
			rivet_term::status::progress("Connecting to ClickHouse", &db_name);

			let clickhouse_user = "bolt";
			let clickhouse_password = ctx
				.read_secret(&["clickhouse", "users", "bolt", "password"])
				.await?;
			let host = conn.clickhouse_host.as_ref().unwrap();
			let (hostname, port) = host.split_once(":").unwrap();

			if let Some(query) = query {
				block_in_place(|| {
					cmd!(
						"clickhouse-client",
						"--host",
						hostname,
						"--port",
						port,
						"--database",
						db_name,
						"--user",
						clickhouse_user,
						"--password",
						clickhouse_password,
						"--query",
						query,
					)
					.run()
				})?;
			} else {
				block_in_place(|| {
					cmd!(
						"clickhouse-client",
						"--host",
						hostname,
						"--port",
						port,
						"--user",
						clickhouse_user,
						"--password",
						clickhouse_password,
					)
					.run()
				})?;
			}
		}
		x @ _ => bail!("cannot migrate this type of service: {x:?}"),
	}

	Ok(())
}
