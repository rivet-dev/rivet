use std::{collections::HashMap, convert::TryInto};

use chirp_workflow::prelude::*;
use fdb_util::{FormalKey, SERIALIZABLE};
use foundationdb as fdb;
use futures_util::{StreamExt, TryStreamExt};

use crate::{
	keys,
	types::{
		Actor, ActorLifecycle, ActorResources, EndpointType, GameGuardProtocol, HostProtocol,
		NetworkMode, Port, Routing,
	},
};

#[derive(Debug, sqlx::FromRow)]
struct ActorRow {
	env_id: Uuid,
	tags: sqlx::types::Json<HashMap<String, String>>,
	resources_cpu_millicores: i64,
	resources_memory_mib: i64,
	selected_resources_cpu_millicores: Option<i64>,
	selected_resources_memory_mib: Option<i64>,
	lifecycle_kill_timeout_ms: i64,
	lifecycle_durable: bool,
	create_ts: i64,
	start_ts: Option<i64>,
	connectable_ts: Option<i64>,
	destroy_ts: Option<i64>,
	client_wan_hostname: Option<String>,
	image_id: Uuid,
	args: sqlx::types::Json<Vec<String>>,
	network_mode: i64,
	environment: sqlx::types::Json<HashMap<String, String>>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct PortIngress {
	pub(crate) port_name: String,
	pub(crate) port_number: Option<i64>,
	ingress_port_number: i64,
	pub(crate) protocol: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct PortHost {
	pub(crate) port_name: String,
	pub(crate) port_number: Option<i64>,
	pub(crate) protocol: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct PortProxied {
	pub(crate) port_name: String,
	pub(crate) source: i64,
}

struct ActorData {
	actor_id: Uuid,
	row: ActorRow,
	port_ingress_rows: Vec<PortIngress>,
	port_host_rows: Vec<PortHost>,
	port_proxied_rows: Vec<PortProxied>,
}

#[derive(Debug)]
pub struct Input {
	pub actor_ids: Vec<Uuid>,

	/// If null, will fall back to the default endpoint type for the datacenter.
	///
	/// If the datacenter has a parent hostname, will use hostname endpoint. Otherwise, will use
	/// path endpoint.
	pub endpoint_type: Option<crate::types::EndpointType>,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<Actor>,
}

#[operation]
pub async fn pegboard_actor_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let actors_with_wf_ids = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			futures_util::stream::iter(input.actor_ids.clone())
				.map(|actor_id| {
					let tx = tx.clone();
					async move {
						let workflow_id_key = keys::actor::WorkflowIdKey::new(actor_id);
						let workflow_id_entry = tx
							.get(&keys::subspace().pack(&workflow_id_key), SERIALIZABLE)
							.await?;

						let Some(workflow_id_entry) = workflow_id_entry else {
							return Ok(None);
						};

						let workflow_id = workflow_id_key
							.deserialize(&workflow_id_entry)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

						Ok(Some((actor_id, workflow_id)))
					}
				})
				.buffer_unordered(1024)
				.try_filter_map(|x| std::future::ready(Ok(x)))
				.try_collect::<Vec<_>>()
				.await
		})
		.await?;

	let actor_data = futures_util::stream::iter(actors_with_wf_ids)
		.map(|(actor_id, workflow_id)| async move {
			let pool = &ctx.sqlite_for_workflow(workflow_id).await?;

			let (actor_row, port_ingress_rows, port_host_rows, port_proxied_rows) = tokio::try_join!(
				sql_fetch_one!(
					[ctx, ActorRow, pool]
					"
					SELECT
						env_id,
						json(tags) AS tags,
						resources_cpu_millicores,
						resources_memory_mib,
						selected_resources_cpu_millicores,
						selected_resources_memory_mib,
						lifecycle_kill_timeout_ms,
						lifecycle_durable,
						create_ts,
						start_ts,
						connectable_ts,
						destroy_ts,
						client_wan_hostname,
						image_id,
						json(args) AS args,
						network_mode,
						json(environment) AS environment
					FROM state
					",
				),
				sql_fetch_all!(
					[ctx, PortIngress, pool]
					"
					SELECT
						port_name,
						port_number,
						ingress_port_number,
						protocol
					FROM ports_ingress
					",
				),
				sql_fetch_all!(
					[ctx, PortHost, pool]
					"
					SELECT port_name, port_number, protocol
					FROM ports_host
					",
				),
				sql_fetch_all!(
					[ctx, PortProxied, pool]
					"
					SELECT port_name, source
					FROM ports_proxied
					",
				),
			)?;

			GlobalResult::Ok(Some(ActorData {
				actor_id,
				row: actor_row,
				port_ingress_rows,
				port_host_rows,
				port_proxied_rows,
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

	let actors = actor_data
		.iter()
		.map(|s| {
			let endpoint_type = input.endpoint_type.unwrap_or_else(|| {
				EndpointType::default_for_guard_public_hostname(&dc.guard_public_hostname)
			});

			let is_connectable = s.row.connectable_ts.is_some();
			let wan_hostname = s.row.client_wan_hostname.clone();

			let ports = s
				.port_ingress_rows
				.iter()
				.map(|port| {
					Ok((
						port.port_name.clone(),
						create_port_ingress(
							s.actor_id,
							is_connectable,
							port,
							unwrap!(GameGuardProtocol::from_repr(port.protocol.try_into()?)),
							endpoint_type,
							&dc.guard_public_hostname,
						)?,
					))
				})
				.chain(s.port_host_rows.iter().map(|host_port| {
					let port_proxied = s.port_proxied_rows.iter().find(|x| {
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
							port_proxied,
						)?,
					))
				}))
				.collect::<GlobalResult<HashMap<_, _>>>()?;

			Ok(Actor {
				actor_id: s.actor_id,
				env_id: s.row.env_id,
				tags: s.row.tags.0.clone(),
				resources: ActorResources {
					cpu_millicores: s
						.row
						.selected_resources_cpu_millicores
						.unwrap_or(s.row.resources_cpu_millicores)
						.try_into()?,
					memory_mib: s
						.row
						.selected_resources_memory_mib
						.unwrap_or(s.row.resources_memory_mib)
						.try_into()?,
				},
				lifecycle: ActorLifecycle {
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

	Ok(Output { actors })
}

pub(crate) fn create_port_ingress(
	actor_id: Uuid,
	is_connectable: bool,
	port: &PortIngress,
	protocol: GameGuardProtocol,
	endpoint_type: EndpointType,
	guard_public_hostname: &cluster::types::GuardPublicHostname,
) -> GlobalResult<Port> {
	let (public_hostname, public_port, public_path) = if is_connectable {
		let (hostname, path) = crate::util::build_actor_hostname_and_path(
			actor_id,
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

pub(crate) fn create_port_host(
	is_connectable: bool,
	wan_hostname: Option<&str>,
	host_port: &PortHost,
	port_proxied: Option<&PortProxied>,
) -> GlobalResult<Port> {
	Ok(Port {
		internal_port: None,
		public_hostname: if is_connectable {
			port_proxied.and(wan_hostname).map(|x| x.to_string())
		} else {
			None
		},
		public_port: if is_connectable {
			port_proxied.map(|x| x.source.try_into()).transpose()?
		} else {
			None
		},
		public_path: None,
		routing: Routing::Host {
			protocol: unwrap!(HostProtocol::from_repr(host_port.protocol.try_into()?)),
		},
	})
}
