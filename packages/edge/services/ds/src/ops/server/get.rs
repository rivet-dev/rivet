use std::{collections::HashMap, convert::TryInto};

use chirp_workflow::prelude::*;
use futures_util::{StreamExt, TryStreamExt};

use crate::types::{
	EndpointType, GameGuardProtocol, HostProtocol, NetworkMode, Port, Routing, Server,
	ServerLifecycle, ServerResources,
};

#[derive(sqlx::FromRow)]
struct ServerRow {
	env_id: Uuid,
	tags: sqlx::types::Json<HashMap<String, String>>,
	resources_cpu_millicores: i64,
	resources_memory_mib: i64,
	lifecycle_kill_timeout_ms: i64,
	lifecycle_durable: bool,
	create_ts: i64,
	start_ts: Option<i64>,
	connectable_ts: Option<i64>,
	destroy_ts: Option<i64>,
	image_id: Uuid,
	args: sqlx::types::Json<Vec<String>>,
	network_mode: i64,
	environment: sqlx::types::Json<HashMap<String, String>>,
}

#[derive(sqlx::FromRow)]
struct PegboardRow {
	client_wan_hostname: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ServerPortIngress {
	port_name: String,
	port_number: Option<i64>,
	ingress_port_number: i64,
	protocol: i64,
}

#[derive(sqlx::FromRow)]
struct ServerPortHost {
	port_name: String,
	protocol: i64,
}

#[derive(sqlx::FromRow)]
struct ProxiedPort {
	port_name: String,
	source: i64,
}

struct ServerData {
	server_id: Uuid,
	row: ServerRow,
	pb_row: PegboardRow,
	port_ingress_rows: Vec<ServerPortIngress>,
	port_host_rows: Vec<ServerPortHost>,
	proxied_port_rows: Vec<ProxiedPort>,
}

#[derive(Debug)]
pub struct Input {
	pub server_ids: Vec<Uuid>,

	/// If null, will fall back to the default endpoint type for the datacenter.
	///
	/// If the datacenter has a parent hostname, will use hostname endpoint. Otherwise, will use
	/// path endpoint.
	pub endpoint_type: Option<crate::types::EndpointType>,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

#[operation]
pub async fn ds_server_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let server_data = futures_util::stream::iter(input.server_ids.clone())
		.map(|server_id| async move {
			let Some(workflow_id) = ctx
				.find_workflow::<crate::workflows::server::Workflow>(("server_id", server_id))
				.await?
			else {
				return GlobalResult::Ok(None);
			};
			let pool = &ctx.sqlite_for_workflow(workflow_id).await?;

			let (server_row, pb_row, port_ingress_rows, port_host_rows, proxied_port_rows) = tokio::try_join!(
				sql_fetch_one!(
					[ctx, ServerRow, pool]
					"
					SELECT
						env_id,
						json(tags) AS tags,
						resources_cpu_millicores,
						resources_memory_mib,
						lifecycle_kill_timeout_ms,
						lifecycle_durable,
						create_ts,
						start_ts,
						connectable_ts,
						destroy_ts,
						image_id,
						json(args) AS args,
						network_mode,
						json(environment) AS environment
					FROM state
					",
				),
				sql_fetch_one!(
					[ctx, PegboardRow, pool]
					"
					SELECT client_wan_hostname
					FROM pegboard
					",
				),
				sql_fetch_all!(
					[ctx, ServerPortIngress, pool]
					"
					SELECT
						port_name,
						port_number,
						ingress_port_number,
						protocol
					FROM server_ports_ingress
					",
				),
				sql_fetch_all!(
					[ctx, ServerPortHost, pool]
					"
					SELECT port_name, protocol
					FROM server_ports_host
					",
				),
				sql_fetch_all!(
					[ctx, ProxiedPort, pool]
					"
					SELECT port_name, source
					FROM server_ports_proxied
					",
				),
			)?;

			Ok(Some(ServerData {
				server_id,
				row: server_row,
				pb_row,
				port_ingress_rows,
				port_host_rows,
				proxied_port_rows,
			}))
		})
		.buffer_unordered(1024)
		.try_filter_map(|x| std::future::ready(Ok(x)))
		.try_collect::<Vec<_>>()
		.await?;

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;
	let dc_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		})
		.await?;
	let dc = unwrap!(dc_res.datacenters.first());

	let servers = server_data
		.iter()
		.map(|s| {
			let endpoint_type =
				input
					.endpoint_type
					.unwrap_or(EndpointType::default_for_guard_public_hostname(
						&dc.guard_public_hostname,
					));

			let is_connectable = s.row.connectable_ts.is_some();
			let wan_hostname = s.pb_row.client_wan_hostname.clone();

			let ports = s
				.port_ingress_rows
				.iter()
				.map(|port| {
					Ok((
						port.port_name.clone(),
						create_port_ingress(
							s.server_id,
							is_connectable,
							port,
							unwrap!(GameGuardProtocol::from_repr(port.protocol.try_into()?)),
							endpoint_type,
							&dc.guard_public_hostname,
						)?,
					))
				})
				.chain(s.port_host_rows.iter().map(|host_port| {
					let proxied_port = s.proxied_port_rows.iter().find(|x| {
						// Transform the port name based on the driver
						let transformed_port_name =
							crate::util::pegboard_normalize_port_name(&host_port.port_name);

						x.port_name == transformed_port_name
					});

					Ok((
						host_port.port_name.clone(),
						create_port_host(
							is_connectable,
							wan_hostname.as_deref(),
							host_port,
							proxied_port,
						)?,
					))
				}))
				.collect::<GlobalResult<HashMap<_, _>>>()?;

			Ok(Server {
				server_id: s.server_id,
				env_id: s.row.env_id,
				tags: s.row.tags.0.clone(),
				resources: ServerResources {
					cpu_millicores: s.row.resources_cpu_millicores.try_into()?,
					memory_mib: s.row.resources_memory_mib.try_into()?,
				},
				lifecycle: ServerLifecycle {
					kill_timeout_ms: s.row.lifecycle_kill_timeout_ms,
					durable: s.row.lifecycle_durable,
				},
				args: s.row.args.0.clone(),
				environment: s.row.environment.0.clone(),
				image_id: s.row.image_id,
				network_mode: unwrap!(NetworkMode::from_repr(s.row.network_mode.try_into()?)),
				network_ports: ports,
				create_ts: s.row.create_ts,
				start_ts: s.row.start_ts,
				connectable_ts: s.row.connectable_ts,
				destroy_ts: s.row.destroy_ts,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { servers })
}

fn create_port_ingress(
	server_id: Uuid,
	is_connectable: bool,
	port: &ServerPortIngress,
	protocol: GameGuardProtocol,
	endpoint_type: EndpointType,
	guard_public_hostname: &cluster::types::GuardPublicHostname,
) -> GlobalResult<Port> {
	let (public_hostname, public_port, public_path) = if is_connectable {
		let (hostname, path) = crate::util::build_ds_hostname_and_path(
			server_id,
			&port.port_name,
			protocol,
			endpoint_type,
			guard_public_hostname,
		)?;
		let port = port.ingress_port_number.try_into()?;
		(Some(hostname), Some(port), path)
	} else {
		(None, None, None)
	};

	Ok(Port {
		internal_port: port.port_number.map(TryInto::try_into).transpose()?,
		public_hostname,
		public_port,
		public_path,
		routing: Routing::GameGuard { protocol },
	})
}

fn create_port_host(
	is_connectable: bool,
	wan_hostname: Option<&str>,
	host_port: &ServerPortHost,
	proxied_port: Option<&ProxiedPort>,
) -> GlobalResult<Port> {
	Ok(Port {
		internal_port: None,
		public_hostname: if is_connectable {
			proxied_port.and(wan_hostname).map(|x| x.to_string())
		} else {
			None
		},
		public_port: if is_connectable {
			proxied_port.map(|x| x.source.try_into()).transpose()?
		} else {
			None
		},
		public_path: None,
		routing: Routing::Host {
			protocol: unwrap!(HostProtocol::from_repr(host_port.protocol.try_into()?)),
		},
	})
}
