use anyhow::*;

use crate::{
	config::{self, service::RuntimeKind},
	context::{ProjectContext, ServiceContext},
	dep::cloudflare,
	utils,
};

pub struct DatabaseConnection {
	pub cockroach_host: Option<String>,
	pub clickhouse_host: Option<String>,

	/// Reference to tunnels that these ports are proxied through. Tunnel will
	/// close on drop.
	_tunnel: Option<cloudflare::Tunnel>,
}

impl DatabaseConnection {
	pub async fn create(
		ctx: &ProjectContext,
		services: &[ServiceContext],
	) -> Result<DatabaseConnection> {
		// Create tunnels for databases
		let mut tunnel_configs = Vec::new();
		let mut cockroach_host = None;
		let mut clickhouse_host = None;
		for svc in services {
			match &svc.config().runtime {
				RuntimeKind::CRDB { .. } => {
					if cockroach_host.is_none() {
						cockroach_host = Some(
							access_service(
								&mut tunnel_configs,
								ctx,
								"sql.cockroach.service.consul:26257",
								(cloudflare::TunnelProtocol::Tcp, "cockroach-sql"),
							)
							.await?,
						);
					}
				}
				RuntimeKind::ClickHouse { .. } => {
					if clickhouse_host.is_none() {
						clickhouse_host = Some(
							access_service(
								&mut tunnel_configs,
								ctx,
								"tcp.clickhouse.service.consul:9000",
								(cloudflare::TunnelProtocol::Tcp, "clickhouse-tcp"),
							)
							.await?,
						);
					}
				}
				x @ _ => bail!("cannot migrate this type of service: {x:?}"),
			}
		}

		// Open tunnels if needed
		let tunnel = if !tunnel_configs.is_empty() {
			Some(cloudflare::Tunnel::open(&ctx, tunnel_configs).await)
		} else {
			None
		};

		Ok(DatabaseConnection {
			cockroach_host,
			clickhouse_host,
			_tunnel: tunnel,
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

/// Creates a tunnel or returns the Consul address depending on the deploy method.
pub async fn access_service(
	tunnel_configs: &mut Vec<cloudflare::TunnelConfig>,
	ctx: &ProjectContext,
	service_hostname: &str,
	(tunnel_protocol, tunnel_name): (cloudflare::TunnelProtocol, &str),
) -> Result<String> {
	match &ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => Ok(service_hostname.into()),
		config::ns::ClusterKind::Distributed { .. } => {
			// Save the tunnel config
			let local_port = utils::pick_port();
			tunnel_configs.push(cloudflare::TunnelConfig::new_with_port(
				tunnel_protocol,
				tunnel_name,
				local_port,
			));

			// Hardcode to the forwarded port
			Ok(format!("127.0.0.1:{local_port}"))
		}
	}
}
