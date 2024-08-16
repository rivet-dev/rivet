use futures_util::FutureExt;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use std::collections::HashMap;

#[derive(sqlx::FromRow)]
struct Server {
	server_id: Uuid,
	env_id: Uuid,
	datacenter_id: Uuid,
	cluster_id: Uuid,
	tags: serde_json::Value,
	resources_cpu_millicores: i64,
	resources_memory_mib: i64,
	kill_timeout_ms: i64,
	create_ts: i64,
	start_ts: Option<i64>,
	connectable_ts: Option<i64>,
	destroy_ts: Option<i64>,
	image_id: Uuid,
	args: Vec<String>,
	network_mode: i64,
	environment: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct DockerPortProtocolGameGuard {
	server_id: Uuid,
	port_name: String,
	port_number: i64,
	gg_port: i64,
	protocol: i64,
}

#[derive(sqlx::FromRow)]
struct DockerPortHost {
	server_id: Uuid,
	port_name: String,
	port_number: i64,
	protocol: i64,
}

#[derive(sqlx::FromRow)]
struct ServerNomad {
	server_id: Uuid,
	nomad_dispatched_job_id: Option<String>,
	nomad_alloc_id: Option<String>,
	nomad_node_id: Option<String>,
	nomad_node_name: Option<String>,
	nomad_node_public_ipv4: Option<String>,
	nomad_node_vlan_ipv4: Option<String>,
	nomad_alloc_plan_ts: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct ServerPort {
	server_id: Uuid,
	nomad_label: String,
	nomad_ip: String,
	nomad_source: i64,
}

#[operation(name = "ds-server-get")]
pub async fn handle(
	ctx: OperationContext<dynamic_servers::server_get::Request>,
) -> GlobalResult<dynamic_servers::server_get::Response> {
	let server_ids = ctx
		.server_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let (server_rows, port_gg_rows, port_host_rows, server_nomad_rows, internal_port_rows) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, Server]
			"
			SELECT
				server_id,
				env_id,
				datacenter_id,
				cluster_id,
				tags,
				resources_cpu_millicores,
				resources_memory_mib,
				kill_timeout_ms,
				create_ts,
				start_ts,
				connectable_ts,
				destroy_ts,
				image_id,
				args,
				network_mode,
				environment
			FROM
				db_ds.servers
			WHERE
				server_id = ANY($1)
			",
			&server_ids,
		),
		sql_fetch_all!(
			[ctx, DockerPortProtocolGameGuard]
			"
			SELECT
				server_id,
				port_name,
				port_number,
				gg_port,
				protocol
			FROM
				db_ds.docker_ports_protocol_game_guard
			WHERE
				server_id = ANY($1)
			",
			&server_ids,
		),
		sql_fetch_all!(
			[ctx, DockerPortHost]
			"
			SELECT
				server_id,
				port_name,
				port_number,
				protocol
			FROM
				db_ds.docker_ports_host
			WHERE
				server_id = ANY($1)
			",
			&server_ids,
		),
		sql_fetch_all!(
			[ctx, ServerNomad]
			"
			SELECT
				server_id,
				nomad_dispatched_job_id,
				nomad_alloc_id,
				nomad_node_id,
				nomad_node_name,
				nomad_node_public_ipv4,
				nomad_node_vlan_ipv4,
				nomad_alloc_plan_ts
			FROM
				db_ds.server_nomad
			WHERE
				server_id = ANY($1)
			",
			&server_ids,
		),
		sql_fetch_all!(
			[ctx, ServerPort]
			"
			SELECT
				server_id,
				nomad_label,
				nomad_ip,
				nomad_source
			FROM
				db_ds.internal_ports
			WHERE
				server_id = ANY($1)
			",
			&server_ids,
		),
	)?;

	let servers_proto = server_rows
		.into_iter()
		.map(|server| {
			let tags: std::collections::HashMap<String, String> =
				serde_json::from_value(server.tags)?;
			let environment: std::collections::HashMap<String, String> =
				serde_json::from_value(server.environment)?;

			let server_nomad = unwrap!(server_nomad_rows
				.iter()
				.find(|x| x.server_id == server.server_id));

			// TODO: Handle timeout to let Traefik pull config
			let is_connectable = server_nomad.nomad_alloc_plan_ts.is_some();

			let ports = port_gg_rows
				.iter()
				.filter(|p| p.server_id == server.server_id)
				.map(|gg_port| {
					GlobalResult::Ok((
						gg_port.port_name.clone(),
						create_port_gg(is_connectable, gg_port, server.datacenter_id)?,
					))
				})
				.chain(
					port_host_rows
						.iter()
						.filter(|p| p.server_id == server.server_id)
						.map(|host_port| {
							let internal_port = internal_port_rows.iter().find(|x| {
								x.server_id == server.server_id
									&& x.nomad_label
										== util_ds::format_nomad_port_label(&host_port.port_name)
							});
							GlobalResult::Ok((
								host_port.port_name.clone(),
								create_port_host(is_connectable, host_port, internal_port)?,
							))
						}),
				)
				.collect::<GlobalResult<HashMap<_, _>>>()?;

			let server_proto = backend::ds::Server {
				server_id: Some(server.server_id.into()),
				env_id: Some(server.env_id.into()),
				datacenter_id: Some(server.datacenter_id.into()),
				cluster_id: Some(server.cluster_id.into()),
				tags,
				resources: Some(backend::ds::ServerResources {
					cpu_millicores: server.resources_cpu_millicores.try_into()?,
					memory_mib: server.resources_memory_mib.try_into()?,
				}),
				kill_timeout_ms: server.kill_timeout_ms,
				args: server.args,
				environment,
				image_id: Some(server.image_id.into()),
				network_mode: server.network_mode.try_into()?,
				network_ports: ports,
				create_ts: server.create_ts,
				start_ts: server.start_ts,
				connectable_ts: server.connectable_ts,
				destroy_ts: server.destroy_ts,
			};

			Ok(server_proto)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(dynamic_servers::server_get::Response {
		servers: servers_proto,
	})
}

fn create_port_gg(
	is_connectable: bool,
	gg_port: &DockerPortProtocolGameGuard,
	datacenter_id: Uuid,
) -> GlobalResult<backend::ds::Port> {
	Ok(backend::ds::Port {
		internal_port: Some(gg_port.port_number.try_into()?),
		public_hostname: if is_connectable {
			Some(util_ds::build_ds_hostname(
				gg_port.server_id,
				&gg_port.port_name,
				datacenter_id,
			)?)
		} else {
			None
		},
		public_port: if is_connectable {
			Some(gg_port.gg_port.try_into()?)
		} else {
			None
		},
		routing: Some(backend::ds::port::Routing::GameGuard(
			backend::ds::GameGuardRouting {
				protocol: gg_port.protocol.try_into()?,
			},
		)),
	})
}

fn create_port_host(
	is_connectable: bool,
	host_port: &DockerPortHost,
	internal_port: Option<&ServerPort>,
) -> GlobalResult<backend::ds::Port> {
	Ok(backend::ds::Port {
		internal_port: Some(host_port.port_number.try_into()?),
		public_hostname: if is_connectable {
			internal_port.map(|x| x.nomad_ip.clone())
		} else {
			None
		},
		public_port: if is_connectable {
			internal_port
				.map(|x| x.nomad_source.try_into())
				.transpose()?
		} else {
			None
		},
		routing: Some(backend::ds::port::Routing::Host(backend::ds::HostRouting {
			protocol: host_port.protocol.try_into()?,
		})),
	})
}
