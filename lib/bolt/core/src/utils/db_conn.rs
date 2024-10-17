use std::{collections::HashMap, sync::Arc};

use anyhow::*;
use indoc::indoc;
use urlencoding::encode;

use crate::{
	config::{
		self,
		project::{SqlService, SqlServiceKind},
	},
	context::ProjectContext,
	dep::terraform,
};

pub struct DatabaseConnections {
	pub redis_hosts: HashMap<String, String>,
	pub cockroach_host: Option<String>,
	pub clickhouse_host: Option<String>,
	pub clickhouse_config: Option<String>,
}

impl DatabaseConnections {
	pub async fn create(
		ctx: &ProjectContext,
		services: &[SqlService],
		forwarded: bool,
	) -> Result<Arc<DatabaseConnections>> {
		match &ctx.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => {
				if forwarded {
					DatabaseConnections::create_local_forwarded(ctx, services).await
				} else {
					DatabaseConnections::create_local(ctx, services).await
				}
			}
			config::ns::ClusterKind::Distributed { .. } => {
				DatabaseConnections::create_distributed(ctx, services).await
			}
		}
	}

	async fn create_local(
		_ctx: &ProjectContext,
		services: &[SqlService],
	) -> Result<Arc<DatabaseConnections>> {
		let mut redis_hosts = HashMap::new();
		let mut cockroach_host = None;
		let mut clickhouse_host = None;
		let mut clickhouse_config = None;

		for svc in services {
			match &svc.kind {
				RuntimeKind::Redis { persistent } => {
					let name = svc.name();

					if !redis_hosts.contains_key(&name) {
						let db_name = if *persistent {
							"persistent"
						} else {
							"ephemeral"
						};

						let host = format!("redis.redis-{db_name}.svc.cluster.local:6379");
						redis_hosts.insert(name, host);
					}
				}
				RuntimeKind::CRDB { .. } => {
					if cockroach_host.is_none() {
						cockroach_host =
							Some("cockroachdb.cockroachdb.svc.cluster.local:26257".to_string());
					}
				}
				RuntimeKind::ClickHouse { .. } => {
					if clickhouse_host.is_none() {
						clickhouse_host =
							Some("clickhouse.clickhouse.svc.cluster.local:9440".to_string());
						clickhouse_config = Some(
							indoc!(
								"
								openSSL:
								  client:
								    caConfig: '/local/clickhouse-ca.crt'
								"
							)
							.to_string(),
						);
					}
				}
				x => bail!("cannot connect to this type of service: {x:?}"),
			}
		}

		Ok(Arc::new(DatabaseConnections {
			redis_hosts,
			cockroach_host,
			clickhouse_host,
			clickhouse_config,
		}))
	}

	async fn create_local_forwarded(
		_ctx: &ProjectContext,
		services: &[SqlService],
	) -> Result<Arc<DatabaseConnections>> {
		let mut redis_hosts = HashMap::new();
		let mut cockroach_host = None;
		let mut clickhouse_host = None;

		for svc in services {
			match &svc.config().runtime {
				RuntimeKind::Redis { .. } => {
					let name = svc.name();

					if !redis_hosts.contains_key(&name) {
						let host = "localhost:6379".to_string();
						redis_hosts.insert(name, host);
					}
				}
				RuntimeKind::CRDB { .. } => {
					if cockroach_host.is_none() {
						cockroach_host = Some("localhost:26257".to_string());
					}
				}
				RuntimeKind::ClickHouse { .. } => {
					if clickhouse_host.is_none() {
						clickhouse_host = Some("localhost:9440".to_string());
					}
				}
				x => bail!("cannot connect to this type of service: {x:?}"),
			}
		}

		Ok(Arc::new(DatabaseConnections {
			redis_hosts,
			cockroach_host,
			clickhouse_host,
			clickhouse_config: None,
		}))
	}

	async fn create_distributed(
		ctx: &ProjectContext,
		services: &[SqlService],
	) -> Result<Arc<DatabaseConnections>> {
		let mut redis_hosts = HashMap::new();
		let mut cockroach_host = None;
		let mut clickhouse_host = None;
		let clickhouse_config = None;

		let redis_data = terraform::output::read_redis(ctx).await;

		for svc in services {
			match &svc.config().runtime {
				RuntimeKind::Redis { persistent } => {
					let name = svc.name();

					if !redis_hosts.contains_key(&name) {
						let db_name = if *persistent {
							"persistent".to_string()
						} else {
							"ephemeral".to_string()
						};

						// Read host and port from terraform
						let hostname = redis_data
							.host
							.get(&db_name)
							.expect("terraform output for redis db not found");
						let port = redis_data
							.port
							.get(&db_name)
							.expect("terraform output for redis db not found");
						let host = format!("{}:{}", *hostname, *port);

						redis_hosts.insert(name, host);
					}
				}
				RuntimeKind::CRDB { .. } => {
					if cockroach_host.is_none() {
						let crdb_data = terraform::output::read_crdb(ctx).await;
						cockroach_host = Some(format!("{}:{}", *crdb_data.host, *crdb_data.port));
					}
				}
				RuntimeKind::ClickHouse { .. } => {
					if clickhouse_host.is_none() {
						let clickhouse_data = terraform::output::read_clickhouse(ctx).await;

						clickhouse_host = Some(format!(
							"{}:{}",
							*clickhouse_data.host, *clickhouse_data.port_native_secure
						));
					}
				}
				x => bail!("cannot connect to this type of service: {x:?}"),
			}
		}

		Ok(Arc::new(DatabaseConnections {
			redis_hosts,
			cockroach_host,
			clickhouse_host,
			clickhouse_config,
		}))
	}
}
