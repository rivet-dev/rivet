use std::collections::HashMap;

use anyhow::*;

use crate::{
	config::service::RuntimeKind,
	context::{ProjectContext, ServiceContext},
	dep,
	utils::{self, DroppablePort},
};

pub struct DatabaseConnection {
	pub redis_hosts: HashMap<String, String>,
	pub cockroach_host: Option<String>,
	pub clickhouse_host: Option<String>,

	_handles: Vec<DroppablePort>,
}

impl DatabaseConnection {
	pub async fn create(
		ctx: &ProjectContext,
		services: &[ServiceContext],
	) -> Result<DatabaseConnection> {
		let mut handles = Vec::new();
		let mut redis_hosts = HashMap::new();
		let mut cockroach_host = None;
		let mut clickhouse_host = None;

		for svc in services {
			match &svc.config().runtime {
				RuntimeKind::Redis { .. } => {
					let name = svc.name();

					if !redis_hosts.contains_key(&name) {
						let port = utils::pick_port();
						let host = format!("127.0.0.1:{port}");

						redis_hosts.insert(name, host);
						handles.push(utils::kubectl_port_forward(
							"redis-master",
							"redis",
							(port, 6379),
						)?);
					}
				}
				RuntimeKind::CRDB { .. } => {
					if cockroach_host.is_none() {
						let port = utils::pick_port();
						cockroach_host = Some(format!("127.0.0.1:{port}"));
						handles.push(utils::kubectl_port_forward(
							"cockroachdb",
							"cockroachdb",
							(port, 26257),
						)?);
					}
				}
				RuntimeKind::ClickHouse { .. } => {
					if clickhouse_host.is_none() {
						let port = utils::pick_port();
						clickhouse_host = Some(format!("127.0.0.1:{port}"));
						handles.push(utils::kubectl_port_forward(
							"clickhouse",
							"clickhouse",
							(port, 9000),
						)?);
					}
				}
				x => bail!("cannot connect to this type of service: {x:?}"),
			}
		}

		// Wait for port forwards to open and check if successful
		DroppablePort::check_all(&handles).await?;

		Ok(DatabaseConnection {
			redis_hosts,
			cockroach_host,
			clickhouse_host,
			_handles: handles,
		})
	}

	/// Returns the URL to use for database migrations.
	pub async fn migrate_db_url(&self, service: &ServiceContext) -> Result<String> {
		let project_ctx = service.project().await;

		match &service.config().runtime {
			RuntimeKind::CRDB { .. } => {
				let db_name = service.crdb_db_name();
				let host = self.cockroach_host.as_ref().unwrap();
				Ok(format!("cockroach://root@{host}/{db_name}?sslmode=disable"))
			}
			RuntimeKind::ClickHouse { .. } => {
				let db_name = service.clickhouse_db_name();
				let clickhouse_user = "bolt";
				let clickhouse_password = project_ctx
					.read_secret(&["clickhouse", "users", "bolt", "password"])
					.await?;
				let host = self.clickhouse_host.as_ref().unwrap();
				Ok(format!("clickhouse://{host}/?database={db_name}&username={clickhouse_user}&password={clickhouse_password}&x-multi-statement=true"))
			}
			x @ _ => bail!("cannot migrate this type of service: {x:?}"),
		}
	}
}
