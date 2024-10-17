use anyhow::*;
use serde_json::json;
use std::{io::Write, path::Path};

pub struct ShellQuery {
	pub svc: String,
	pub query: Option<String>,
}

pub struct ShellContext<'a> {
	pub queries: &'a [ShellQuery],
}

pub async fn redis_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	for ShellQuery { svc, query } in queries {
		let url_var = format!("REDIS_URL_{}", svc.to_uppercase().replace("-", "_"));
		let url = std::env::var(&url_var).context(format!("{url_var} not set"))?;

		tracing::info!(?svc, "connecting to bolt");

		if query.is_some() {
			todo!("cannot pass query to redis shell at the moment");
		}

		let parsed_url = url::Url::parse(&url)?;
		let hostname = parsed_url.host_str().context("Missing hostname")?;
		let port = parsed_url.port().unwrap_or(6379);
		let username = parsed_url.username();

		let mut cmd = std::process::Command::new("redis-cli");
		cmd.args(&[
			"-h",
			hostname,
			"-p",
			&port.to_string(),
			"--user",
			username,
			"-c",
			"--tls",
		]);

		let ca_path = format!("/usr/local/share/ca-certificates/redis-{svc}-ca.crt");
		if Path::new(&ca_path).exists() {
			cmd.arg("--cacert").arg(&ca_path);
		}

		if let Some(password) = parsed_url.password() {
			cmd.env("REDISCLI_AUTH", password);
		}

		dbg!(&cmd);

		cmd.status()?;
	}

	Ok(())
}

pub async fn cockroachdb_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	tracing::info!("connecting to cockroachdb");

	// Combine all queries into one command
	for ShellQuery {
		svc: db_name,
		query,
	} in queries
	{
		let url = rivet_pools::crdb_url_from_env()?.context("missing cockroachdb url")?;
		let mut parsed_url = url::Url::parse(&url)?;
		parsed_url.set_path(&format!("/{}", db_name));

		let ca_path = "/usr/local/share/ca-certificates/crdb-ca.crt";
		if Path::new(&ca_path).exists() {
			parsed_url.set_query(Some(&format!("sslmode=verify-ca&sslrootcert={ca_path}")));
		} else {
			parsed_url.set_query(None);
		}

		let db_url = parsed_url.to_string();

		let mut cmd = std::process::Command::new("psql");
		cmd.arg(&db_url);

		if let Some(query) = query {
			cmd.args(&["-c", query]);
		}

		// See https://github.com/cockroachdb/cockroach/issues/37129#issuecomment-600115995
		cmd.env("PGCLIENTENCODING", "utf-8");

		dbg!(&cmd);

		cmd.status()?;
	}

	Ok(())
}

pub async fn clickhouse_shell(shell_ctx: ShellContext<'_>) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	tracing::info!("connecting to clickhouse");

	for ShellQuery {
		svc: db_name,
		query,
	} in queries
	{
		let url = rivet_pools::clickhouse_url_from_env()?.context("missing clickhouse url")?;
		let parsed_url = url::Url::parse(&url)?;

		let hostname = parsed_url.host_str().unwrap_or("localhost");
		let port = parsed_url.port().unwrap_or(9440).to_string();
		let user = parsed_url.username();
		let password = parsed_url.password().unwrap_or("");

		let ca_path = "/usr/local/share/ca-certificates/clickhouse-ca.crt";
		let config = json!({
			"user": user,
			"password": password,
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
			.arg(user)
			.arg("--password")
			.arg(password)
			.arg("--secure")
			.arg("--database")
			.arg(db_name)
			.arg("--config-file")
			.arg(config_file.path());

		if let Some(query) = query {
			cmd.arg("--multiquery").arg(query);
		}

		dbg!(&cmd);

		cmd.status()?;
	}

	Ok(())
}
