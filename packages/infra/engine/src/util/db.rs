use std::{path::Path, result::Result::Ok, str::FromStr};

use anyhow::*;
use rivet_util::Id;
use serde_json::json;

pub struct ShellQuery {
	pub svc: String,
	pub query: Option<String>,
}

pub struct ShellContext<'a> {
	pub queries: &'a [ShellQuery],
}

pub async fn clickhouse_shell(
	config: rivet_config::Config,
	shell_ctx: ShellContext<'_>,
) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	tracing::info!("connecting to clickhouse");

	for ShellQuery {
		svc: db_name,
		query,
	} in queries
	{
		let clickhouse_config = config.clickhouse().context("clickhouse disabled")?;
		let parsed_url = clickhouse_config.native_url.clone();

		let hostname = parsed_url.host_str().unwrap_or("localhost");
		let port = parsed_url.port().unwrap_or(9440).to_string();

		let ca_path = "/usr/local/share/ca-certificates/clickhouse-ca.crt";
		let config = json!({
			"user": clickhouse_config.username,
			"password": clickhouse_config.password.as_ref().map(|x| x.read().clone()),
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

		let mut config_file = tempfile::Builder::new().suffix(".yaml").tempfile()?;
		serde_yaml::to_writer(&mut config_file, &config)?;

		let mut cmd = std::process::Command::new("clickhouse-client");
		cmd.arg("--host")
			.arg(hostname)
			.arg("--port")
			.arg(&port)
			.arg("--user")
			.arg(&clickhouse_config.username)
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

pub async fn wf_sqlite_shell(
	config: rivet_config::Config,
	shell_ctx: ShellContext<'_>,
	_internal: bool,
) -> Result<()> {
	let ShellContext { queries, .. } = shell_ctx;

	let _pools = rivet_pools::Pools::new(config.clone()).await?;

	// Combine all queries into one command
	for ShellQuery {
		svc: workflow_id,
		query: _query,
	} in queries
	{
		let _workflow_id = Id::from_str(workflow_id).context("could not parse input as Id")?;

		todo!();
	}

	Ok(())
}
