use anyhow::*;
use serde_json::json;
use std::path::Path;

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
		cmd.args(&["-h", hostname, "-p", &port.to_string(), "-c", "--tls"]);
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
			cmd.args(&["-c", query]);
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
		let parsed_url = clickhouse_config.url.clone();

		let hostname = parsed_url.host_str().unwrap_or("localhost");
		let port = parsed_url.port().unwrap_or(9440).to_string();

		let ca_path = "/usr/local/share/ca-certificates/clickhouse-ca.crt";
		let config = json!({
			"user": clickhouse_config.username,
			"password": clickhouse_config.password.as_ref().map(|x| x.read().clone()),
			"secure": true,
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

		let mut config_file = tempfile::NamedTempFile::new()?;
		serde_yaml::to_writer(&mut config_file, &config)?;

		let mut cmd = std::process::Command::new("clickhouse-client");
		cmd.arg("--host")
			.arg(hostname)
			.arg("--port")
			.arg(&port)
			.arg("--user")
			.arg(&clickhouse_config.username)
			.arg("--secure")
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
