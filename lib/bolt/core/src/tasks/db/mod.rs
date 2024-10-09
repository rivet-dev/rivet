use anyhow::*;
use duct::cmd;
use indoc::formatdoc;
use rand::Rng;
use serde_json::json;
use tokio::{io::AsyncWriteExt, process::Command, task::block_in_place};

use crate::{
	config::{self, service::RuntimeKind},
	context::{ProjectContext, ServiceContext},
	dep,
	utils::{self, db_conn::DatabaseConnections},
};

pub mod sqlx;

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
	pub forwarded: bool,
	pub conn: &'a DatabaseConnections,
	pub queries: &'a [ShellQuery],
	pub log_type: LogType,
}

pub async fn shell(
	ctx: &ProjectContext,
	svc: &ServiceContext,
	query: Option<&str>,
	forwarded: bool,
) -> Result<()> {
	let forwarded = forwarded
		&& matches!(
			&ctx.ns().cluster.kind,
			config::ns::ClusterKind::SingleNode { .. }
		);

	let conn = DatabaseConnections::create(ctx, &[svc.clone()], forwarded).await?;
	let shell_ctx = ShellContext {
		ctx,
		forwarded,
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
		x => bail!("cannot migrate this type of service: {x:?}"),
	}

	Ok(())
}

async fn redis_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext {
		ctx,
		forwarded,
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
	let (hostname, port) = host.split_once(':').unwrap();

	// Read auth secrets
	let (username, password) = match ctx.ns().redis.provider {
		config::ns::RedisProvider::Kubernetes {} | config::ns::RedisProvider::Aiven { .. } => (
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
	let mount_ca = matches!(
		ctx.ns().redis.provider,
		config::ns::RedisProvider::Kubernetes {}
	);

	if let LogType::Default = log_type {
		rivet_term::status::progress("Connecting to Redis", db_name);
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

	let mut cmd = formatdoc!(
		"
		redis-cli \
		-h {hostname} \
		-p {port} \
		--user {username} \
		-c",
	);

	if forwarded {
		let port = utils::kubectl_port_forward(
			ctx,
			&format!("redis-{db_name}"),
			"svc/redis",
			(6379, 6379),
		)?;
		port.check().await?;

		block_in_place(|| cmd!("bash", "-c", cmd).run())?;
	} else {
		cmd.push_str(" --tls");

		if mount_ca {
			cmd.push_str(" --cacert /local/redis-ca.crt");
		}

		let pod_spec = json!({
			"restartPolicy": "Never",
			"terminationGracePeriodSeconds": 0,
			"containers": [
				{
					"name": "redis",
					"image": REDIS_IMAGE,
					"command": ["sleep", "10000"],
					"env": env,
					"stdin": true,
					"stdinOnce": true,
					"tty": true,
					"volumeMounts": if mount_ca {
						json!([{
							"name": "redis-ca",
							"mountPath": "/local/redis-ca.crt",
							"subPath": "redis-ca.crt"
						}])
					} else {
						json!([])
					}
				}
			],
			"volumes": if mount_ca {
				json!([{
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
				}])
			} else {
				json!([])
			}
		});

		let pod_name = format!("redis-{db_name}-sh-persistent");
		start_persistent_pod(ctx, "Redis", &pod_name, pod_spec).await?;

		// Connect to persistent pod
		block_in_place(|| {
			cmd!(
				"kubectl",
				"exec",
				format!("job/{pod_name}"),
				"-it",
				"-n",
				"bolt",
				"--",
				"sh",
				"-c",
				cmd,
			)
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.run()
		})?;
	}

	Ok(())
}

pub async fn crdb_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext {
		ctx,
		forwarded,
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
		let host = conn.cockroach_host.as_ref().unwrap();
		let username = ctx.read_secret(&["crdb", "username"]).await?;
		let password = ctx.read_secret(&["crdb", "password"]).await?;
		let mut db_url = format!("postgres://{}:{}@{}/{}", username, password, host, db_name);

		// Add SSL
		if !forwarded {
			db_url.push_str("?sslmode=verify-ca&sslrootcert=/local/crdb-ca.crt");
		}

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

	if forwarded {
		let port =
			utils::kubectl_port_forward(ctx, "cockroachdb", "svc/cockroachdb", (26257, 26257))?;
		port.check().await?;

		block_in_place(|| cmd!("bash", "-c", query_cmd).run())?;
	} else {
		let pod_spec = json!({
			"restartPolicy": "Never",
			"terminationGracePeriodSeconds": 0,
			"containers": [
				{
					"name": "postgres",
					"image": "postgres",
					"command": ["sleep", "10000"],
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
		});

		let pod_name = "crdb-sh-persistent";
		start_persistent_pod(ctx, "Cockroach", pod_name, pod_spec).await?;

		// Connect to persistent pod
		block_in_place(|| {
			cmd!(
				"kubectl",
				"exec",
				format!("job/{pod_name}"),
				"-it",
				"-n",
				"bolt",
				"--",
				"sh",
				"-c",
				query_cmd,
			)
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.run()
		})?;
	}

	Ok(())
}

// `no_db` connects without specifying a database
pub async fn clickhouse_shell(shell_ctx: ShellContext<'_>, no_db: bool) -> Result<()> {
	let ShellContext {
		ctx,
		forwarded,
		conn,
		queries,
		log_type,
	} = shell_ctx;

	if let LogType::Default = log_type {
		rivet_term::status::progress("Connecting to ClickHouse", "");
	}

	// Combine all queries into one command
	let mut query_cmd = String::new();
	for ShellQuery { svc, query } in queries {
		let db_name = svc.clickhouse_db_name();
		let user = "default";
		let password = ctx
			.read_secret(&["clickhouse", "users", "default", "password"])
			.await?;
		let host = conn.clickhouse_host.as_ref().unwrap();
		let (hostname, port) = host.split_once(':').unwrap();

		let mut cmd = formatdoc!(
			"
			clickhouse-client \
				--host {hostname} \
				--port {port} \
				--user {user} \
				--password {password} \
				--secure"
		);

		if !forwarded {
			cmd.push_str(" --config-file /local/config.yml ");
		}
		if !no_db {
			cmd.push_str(&format!(" --database {db_name}"));
		}
		if let Some(query) = query {
			cmd.push_str(&format!(" --multiquery '{}'", query.replace('\'', "'\\''")));
		}

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

	// TODO: Does not work when forwarded, not sure why
	if forwarded {
		let port = utils::kubectl_port_forward(ctx, "clickhouse", "svc/clickhouse", (9440, 9440))?;
		port.check().await?;

		block_in_place(|| cmd!("bash", "-c", query_cmd).run())?;
	} else {
		let pod_spec = json!({
			"restartPolicy": "Never",
			"terminationGracePeriodSeconds": 0,
			"containers": [
				{
					"name": "clickhouse",
					"image": "clickhouse/clickhouse-server",
					"command": ["sleep", "10000"],
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

		let pod_name = "clickhouse-sh-persistent";
		start_persistent_pod(ctx, "ClickHouse", pod_name, pod_spec).await?;

		// Connect to persistent pod
		block_in_place(|| {
			cmd!(
				"kubectl",
				"exec",
				format!("job/{pod_name}"),
				"-it",
				"-n",
				"bolt",
				"--",
				"sh",
				"-c",
				query_cmd,
			)
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.run()
		})?;
	}

	Ok(())
}

pub async fn start_persistent_pod(
	ctx: &ProjectContext,
	title: &str,
	pod_name: &str,
	pod_spec: serde_json::Value,
) -> Result<()> {
	let res = block_in_place(|| {
		cmd!(
			"kubectl",
			"get",
			"pod",
			format!("--selector=job-name={pod_name}"),
			"-n",
			"bolt",
			"--ignore-not-found"
		)
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.read()
	})?;
	let persistent_pod_exists = !res.is_empty();

	if !persistent_pod_exists {
		rivet_term::status::progress(&format!("Creating persistent {title} pod"), "");

		let spec = json!({
			"apiVersion": "batch/v1",
			"kind": "Job",
			"metadata": {
				"name": pod_name,
				"namespace": "bolt",
				"labels": {
					"app.kubernetes.io/name": pod_name
				}
			},
			"spec": {
				"ttlSecondsAfterFinished": 5,
				"completions": 1,
				"backoffLimit": 0,
				"template": {
					"metadata": {
						"labels": {
							"app.kubernetes.io/name": pod_name,
						},
					},
					"spec": pod_spec,
				}
			}
		});

		dep::k8s::cli::apply_specs(ctx, vec![spec]).await?;

		// Wait for ready
		let label = format!("app.kubernetes.io/name={pod_name}");
		let status = Command::new("kubectl")
			.args([
				"wait",
				"--for=condition=Ready",
				"pod",
				"--selector",
				&label,
				"-n",
				"bolt",
			])
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.stdout(std::process::Stdio::null())
			.status()
			.await?;
		if !status.success() {
			bail!("failed to check pod readiness");
		}
	}

	Ok(())
}

// Generates a pod name for the shell with a random hash at the end
pub fn shell_name(name: &str) -> String {
	let hash = rand::thread_rng().gen_range::<usize, _>(0..9999);

	format!("{name}-sh-{hash}")
}
