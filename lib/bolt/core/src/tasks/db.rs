use anyhow::*;
use duct::cmd;
use indoc::formatdoc;
use rand::Rng;
use serde_json::json;
use tokio::{io::AsyncWriteExt, task::block_in_place};

use crate::{
	config::{self, service::RuntimeKind},
	context::{ProjectContext, ServiceContext},
	utils::db_conn::DatabaseConnections,
};

const REDIS_IMAGE: &str = "ghcr.io/rivet-gg/redis:cc3241e";

pub enum LogType {
	Default,
	Migration,
}

pub struct ShellQuery {
	pub svc: ServiceContext,
	pub query: Option<String>,
}

pub struct ShellContext<'a> {
	pub ctx: &'a ProjectContext,
	pub conn: &'a DatabaseConnections,
	pub queries: &'a [ShellQuery],
	pub log_type: LogType,
}

pub async fn shell(ctx: &ProjectContext, svc: &ServiceContext, query: Option<&str>) -> Result<()> {
	let conn = DatabaseConnections::create(ctx, &[svc.clone()]).await?;
	let shell_ctx = ShellContext {
		ctx,
		conn: &conn,
		queries: &[ShellQuery {
			svc: svc.clone(),
			query: query.map(|s| s.to_string()),
		}],
		log_type: LogType::Default,
	};

	match &svc.config().runtime {
		RuntimeKind::Redis { .. } => redis_shell(shell_ctx).await?,
		RuntimeKind::CRDB { .. } => crdb_shell(shell_ctx).await?,
		RuntimeKind::ClickHouse { .. } => clickhouse_shell(shell_ctx, false).await?,
		x @ _ => bail!("cannot migrate this type of service: {x:?}"),
	}

	Ok(())
}

async fn redis_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext {
		ctx,
		conn,
		queries,
		log_type,
	} = shell_ctx;

	// TODO: Implement multiple queries
	let ShellQuery { svc, query } = queries.first().unwrap();

	let db_name = if let RuntimeKind::Redis { persistent } = svc.config().runtime {
		if persistent {
			"persistent"
		} else {
			"ephemeral"
		}
	} else {
		// In `redis_shell`
		unreachable!();
	};
	let host = conn.redis_hosts.get(&svc.name()).unwrap();
	let (hostname, port) = host.split_once(":").unwrap();

	// Read auth secrets
	let (username, password) = match ctx.ns().redis.provider {
		config::ns::RedisProvider::Kubernetes {} => (
			ctx.read_secret(&["redis", &db_name, "username"]).await?,
			ctx.read_secret_opt(&["redis", &db_name, "password"])
				.await?,
		),
		config::ns::RedisProvider::Aws {} => {
			let db_name = format!("rivet-{}-{}", ctx.ns_id(), db_name);
			let username = ctx.read_secret(&["redis", &db_name, "username"]).await?;
			let password = ctx
				.read_secret_opt(&["redis", &db_name, "password"])
				.await?;

			(username, password)
		}
	};

	if let LogType::Default = log_type {
		rivet_term::status::progress("Connecting to Redis", &db_name);
	}

	if query.is_some() {
		todo!("cannot pass query to redis shell at the moment");
	}

	let env = if let Some(password) = password {
		vec![json!({
			"name": "REDISCLI_AUTH",
			"value": password,
		})]
	} else {
		Vec::new()
	};
	let cmd = formatdoc!(
		"
		sleep 1 &&
		redis-cli \
		-h {hostname} \
		-p {port} \
		--user {username} \
		-c \
		--tls --cacert /local/redis-ca.crt
		"
	);
	let overrides = json!({
		"apiVersion": "v1",
		"metadata": {
			"namespace": "bolt",
		},
		"spec": {
			"containers": [
				{
					"name": "redis",
					"image": REDIS_IMAGE,
					// "command": ["redis-cli"],
					// "args": [
					// 	"-h", hostname,
					// 	"-p", port,
					// 	"--user", username,
					// 	"-c",
					// 	"--tls",
					// 	"--cacert", "/local/redis-ca.crt"
					// ],
					"command": ["sh", "-c"],
					"args": [cmd],
					"env": env,
					"stdin": true,
					"stdinOnce": true,
					"tty": true,
					"volumeMounts": [{
						"name": "redis-ca",
						"mountPath": "/local/redis-ca.crt",
						"subPath": "redis-ca.crt"
					}]
				}
			],
			"volumes": [{
				"name": "redis-ca",
				"configMap": {
					"name": format!("redis-{}-ca", db_name),
					"defaultMode": 420,
					// Distributed clusters don't need a CA for redis
					"optional": true,
					"items": [
						{
							"key": "ca.crt",
							"path": "redis-ca.crt"
						}
					]
				}
			}]
		}
	});

	block_in_place(|| {
		cmd!(
			"kubectl",
			"run",
			"-itq",
			"--rm",
			"--restart=Never",
			format!("--image={REDIS_IMAGE}"),
			"--namespace",
			"bolt",
			format!("--overrides={overrides}"),
			shell_name("redis"),
		)
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.run()
	})?;

	Ok(())
}

pub async fn crdb_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext {
		ctx,
		conn,
		queries,
		log_type,
	} = shell_ctx;

	if let LogType::Default = log_type {
		rivet_term::status::progress("Connecting to Cockroach", "");
	}

	// Combine all queries into one command
	let mut query_cmd = String::new();
	for ShellQuery { svc, query } in queries {
		let db_name = svc.crdb_db_name();
		let conn = conn.cockroach_host.as_ref().unwrap();
		let username = ctx.read_secret(&["crdb", "username"]).await?;
		let password = ctx.read_secret(&["crdb", "password"]).await?;
		let db_url = format!(
			"postgres://{}:{}@{}/{}?sslmode=verify-ca&sslrootcert=/local/crdb-ca.crt",
			username, password, conn, db_name
		);

		let query = if let Some(query) = query {
			format!("-c '{}'", query.replace('\'', "'\\''"))
		} else {
			"".to_string()
		};
		let cmd = format!("psql \"{db_url}\" {query}");

		if let LogType::Migration = log_type {
			// Append command
			if !query_cmd.is_empty() {
				query_cmd.push_str(" && ");
			}
			query_cmd.push_str(&format!("echo Querying {}", svc.name()));
		}

		// Append command
		if !query_cmd.is_empty() {
			query_cmd.push_str(" && ");
		}
		query_cmd.push_str(&cmd);
	}

	let overrides = json!({
		"apiVersion": "v1",
		"metadata": {
			"namespace": "bolt",
		},
		"spec": {
			"containers": [
				{
					"name": "postgres",
					"image": "postgres",
					"command": ["sh", "-c"],
					"args": [query_cmd],
					"env": [
						// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
						{
							"name": "PGCLIENTENCODING",
							"value": "utf-8",
						}
					],
					"stdin": true,
					"stdinOnce": true,
					"tty": true,
					"volumeMounts": [{
						"name": "crdb-ca",
						"mountPath": "/local/crdb-ca.crt",
						"subPath": "crdb-ca.crt"
					}]
				}
			],
			"volumes": [{
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
			}]
		}
	});

	block_in_place(|| {
		cmd!(
			"kubectl",
			"run",
			"-itq",
			"--rm",
			"--restart=Never",
			"--image=postgres",
			"--namespace",
			"bolt",
			format!("--overrides={overrides}"),
			shell_name("crdb"),
		)
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.run()
	})?;

	Ok(())
}

// `no_db` connects without specifying a database
pub async fn clickhouse_shell(shell_ctx: ShellContext<'_>, no_db: bool) -> Result<()> {
	let ShellContext {
		ctx,
		conn,
		queries,
		log_type,
	} = shell_ctx;

	if let LogType::Default = log_type {
		rivet_term::status::progress("Connecting to ClickHouse", "");
	}

	// Combine all queries into one command
	let mut query_cmd = "sleep 1".to_string();
	for ShellQuery { svc, query } in queries {
		let db_name = svc.clickhouse_db_name();
		let user = "default";
		let password = ctx
			.read_secret(&["clickhouse", "users", "default", "password"])
			.await?;
		let host = conn.clickhouse_host.as_ref().unwrap();
		let (hostname, port) = host.split_once(":").unwrap();

		let db_flag = if no_db {
			"".to_string()
		} else {
			format!("--database {db_name}")
		};
		let query = if let Some(query) = query {
			format!("--multiquery '{}'", query.replace('\'', "'\\''"))
		} else {
			"".to_string()
		};
		let cmd = formatdoc!(
			"
			clickhouse-client \
				--secure \
				--config-file /local/config.yml \
				--host {hostname} \
				--port {port} \
				--user {user} \
				--password {password} {db_flag} {query}
			"
		);

		if let LogType::Migration = log_type {
			// Append command
			if !query_cmd.is_empty() {
				query_cmd.push_str(" && ");
			}
			query_cmd.push_str(&format!("echo Querying {}", svc.name()));
		}

		// Append command
		if !query_cmd.is_empty() {
			query_cmd.push_str(" && ");
		}
		query_cmd.push_str(cmd.trim());
	}

	let overrides = json!({
		"apiVersion": "v1",
		"metadata": {
			"namespace": "bolt",
		},
		"spec": {
			"containers": [
				{
					"name": "clickhouse",
					"image": "clickhouse/clickhouse-server",
					"command": ["sh", "-c"],
					"args": [query_cmd],
					"stdin": true,
					"stdinOnce": true,
					"tty": true,
					"volumeMounts": [
						{
							"name": "clickhouse-ca",
							"mountPath": "/local/clickhouse-ca.crt",
							"subPath": "clickhouse-ca.crt"
						},
						{
							"name": "clickhouse-config",
							"mountPath": "/local/config.yml",
							"subPath": "config.yml",
						}
					]
				}
			],
			"volumes": [{
				"name": "clickhouse-ca",
				"configMap": {
					"name": "clickhouse-ca",
					"defaultMode": 420,
					// Distributed clusters don't need a CA for clickhouse
					"optional": true,
					"items": [
						{
							"key": "ca.crt",
							"path": "clickhouse-ca.crt"
						}
					]
				}
			}, {
				"name": "clickhouse-config",
				"configMap": {
					"name": "clickhouse-config",
					"defaultMode": 420,
					"optional": true
				}
			}]
		}
	});

	// Apply clickhouse config to K8s
	if let Some(config) = &conn.clickhouse_config {
		let spec = serde_json::to_vec(&json!({
			"kind": "ConfigMap",
			"apiVersion": "v1",
			"metadata": {
				"name": "clickhouse-config",
				"namespace": "bolt"
			},
			"data": {
				"config.yml": config
			}
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
	}

	block_in_place(|| {
		cmd!(
			"kubectl",
			"run",
			"-itq",
			"--rm",
			"--restart=Never",
			"--image=clickhouse/clickhouse-server",
			"--namespace",
			"bolt",
			format!("--overrides={overrides}"),
			shell_name("clickhouse"),
		)
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.run()
	})?;

	Ok(())
}

// Generates a pod name for the shell with a random hash at the end
pub fn shell_name(name: &str) -> String {
	let hash = rand::thread_rng().gen_range::<usize, _>(0..9999);

	format!("{name}-sh-{hash}")
}
