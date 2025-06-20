use std::time::Instant;

use build::types::BuildKind;
use chirp_workflow::prelude::*;
use fdb_util::{end_of_key_range, FormalKey, SERIALIZABLE, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
};
use futures_util::{FutureExt, TryStreamExt};
use rivet_api::models::actors_endpoint_type;
use sqlx::Acquire;

use super::{
	destroy::{self, KillCtx},
	setup, Destroy, Input, ACTOR_START_THRESHOLD_MS, BASE_RETRY_TIMEOUT_MS,
	RETRY_RESET_DURATION_MS,
};
use crate::{
	keys, metrics,
	ops::actor::get,
	protocol,
	types::{EndpointType, GameGuardProtocol, HostProtocol, NetworkMode, Port, Routing},
	workflows::client::CLIENT_ELIGIBLE_THRESHOLD_MS,
};

#[derive(Deserialize, Serialize)]
pub struct State {
	pub generation: u32,

	pub client_id: Uuid,
	pub client_workflow_id: Uuid,
	pub image_id: Option<Uuid>,

	pub drain_timeout_ts: Option<i64>,
	pub gc_timeout_ts: Option<i64>,

	#[serde(default)]
	reschedule_state: RescheduleState,
}

impl State {
	pub fn new(client_id: Uuid, client_workflow_id: Uuid, image_id: Uuid) -> Self {
		State {
			generation: 0,
			client_id,
			client_workflow_id,
			image_id: Some(image_id),
			drain_timeout_ts: None,
			gc_timeout_ts: Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS),
			reschedule_state: RescheduleState::default(),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct StateRes {
	pub kill: Option<KillCtx>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct RescheduleState {
	last_retry_ts: i64,
	retry_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateClientInput {
	client_id: Uuid,
	client_workflow_id: Uuid,
}

#[activity(UpdateClient)]
async fn update_client(ctx: &ActivityCtx, input: &UpdateClientInput) -> GlobalResult<()> {
	let client_pool = ctx.sqlite_for_workflow(input.client_workflow_id).await?;
	let pool = ctx.sqlite().await?;

	let (client_wan_hostname,) = sql_fetch_one!(
		[ctx, (String,), client_pool]
		"
		SELECT config->'network'->>'wan_hostname' AS wan_hostname
		FROM state
		",
	)
	.await?;

	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET
			client_id = ?,
			client_workflow_id = ?,
			client_wan_hostname = ?
		",
		input.client_id,
		input.client_workflow_id,
		&client_wan_hostname,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchPortsInput {
	actor_id: Uuid,
	endpoint_type: Option<EndpointType>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FetchPortsOutput {
	ports: Vec<FetchedPort>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FetchedPort {
	name: String,
	port_number: Option<u16>,
	port: Port,
}

#[activity(FetchPorts)]
async fn fetch_ports(ctx: &ActivityCtx, input: &FetchPortsInput) -> GlobalResult<FetchPortsOutput> {
	let pool = ctx.sqlite().await?;

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let ((wan_hostname,), port_ingress_rows, port_host_rows, dc_res) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (Option<String>,), &pool]
			"
			SELECT client_wan_hostname
			FROM state
			",
		),
		sql_fetch_all!(
			[ctx, get::PortIngress, &pool]
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
			[ctx, get::PortHost, &pool]
			"
			SELECT port_name, port_number, protocol
			FROM ports_host
			",
		),
		ctx.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![dc_id],
		}),
	)?;

	let dc = unwrap!(dc_res.datacenters.first());

	let endpoint_type = input.endpoint_type.unwrap_or_else(|| {
		EndpointType::default_for_guard_public_hostname(&dc.guard_public_hostname)
	});

	let ports = port_ingress_rows
		.into_iter()
		.map(|row| {
			let port = get::create_port_ingress(
				input.actor_id,
				&row,
				unwrap!(GameGuardProtocol::from_repr(row.protocol.try_into()?)),
				endpoint_type,
				&dc.guard_public_hostname,
			)?;

			Ok(FetchedPort {
				name: row.port_name,
				port_number: row.port_number.map(TryInto::try_into).transpose()?,
				port,
			})
		})
		.chain(port_host_rows.into_iter().map(|row| {
			let port = get::create_port_host(
				true,
				wan_hostname.as_deref(),
				&row,
				// Placeholder, will be replaced in the isolate runner when building
				// metadata
				Some(&get::PortProxied {
					port_name: String::new(),
					source: 0,
				}),
			)?;

			Ok(FetchedPort {
				name: row.port_name,
				port_number: row.port_number.map(TryInto::try_into).transpose()?,
				port,
			})
		}))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(FetchPortsOutput { ports })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct AllocateActorInputV1 {
	actor_id: Uuid,
	build_kind: BuildKind,
	resources: protocol::Resources,
}

#[activity(AllocateActorV1)]
async fn allocate_actor(
	ctx: &ActivityCtx,
	input: &AllocateActorInputV1,
) -> GlobalResult<Option<AllocateActorOutputV2>> {
	AllocateActorV2::run(
		ctx,
		&AllocateActorInputV2 {
			actor_id: input.actor_id,
			generation: 0,
			build_kind: input.build_kind,
			resources: input.resources.clone(),
		},
	)
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct AllocateActorInputV2 {
	actor_id: Uuid,
	generation: u32,
	build_kind: BuildKind,
	resources: protocol::Resources,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllocateActorOutputV2 {
	pub client_id: Uuid,
	pub client_workflow_id: Uuid,
}

#[activity(AllocateActorV2)]
async fn allocate_actor_v2(
	ctx: &ActivityCtx,
	input: &AllocateActorInputV2,
) -> GlobalResult<Option<AllocateActorOutputV2>> {
	let client_flavor = match input.build_kind {
		BuildKind::DockerImage | BuildKind::OciBundle => protocol::ClientFlavor::Container,
		BuildKind::JavaScript => protocol::ClientFlavor::Isolate,
	};
	let memory_mib = input.resources.memory / 1024 / 1024;

	let start_instant = Instant::now();

	let res = ctx
		.fdb()
		.await?
		.run(|tx, _mc| async move {
			let ping_threshold_ts = util::timestamp::now() - CLIENT_ELIGIBLE_THRESHOLD_MS;

			// Select a range that only includes clients that have enough remaining mem to allocate this actor
			let start = keys::subspace().pack(
				&keys::datacenter::ClientsByRemainingMemKey::subspace_with_mem(
					client_flavor,
					memory_mib,
				),
			);
			let client_allocation_subspace =
				keys::datacenter::ClientsByRemainingMemKey::subspace(client_flavor);
			let end = keys::subspace()
				.subspace(&client_allocation_subspace)
				.range()
				.1;

			let mut stream = tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::Iterator,
					// Containers bin pack so we reverse the order
					reverse: matches!(client_flavor, protocol::ClientFlavor::Container),
					..(start, end).into()
				},
				// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the keys, just
				// the one we choose
				SNAPSHOT,
			);

			loop {
				let Some(entry) = stream.try_next().await? else {
					return Ok(None);
				};

				let old_allocation_key = keys::subspace()
					.unpack::<keys::datacenter::ClientsByRemainingMemKey>(entry.key())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Scan by last ping
				if old_allocation_key.last_ping_ts < ping_threshold_ts {
					continue;
				}

				let client_workflow_id = old_allocation_key
					.deserialize(entry.value())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Add read conflict only for this key
				tx.add_conflict_range(
					entry.key(),
					&end_of_key_range(entry.key()),
					ConflictRangeType::Read,
				)?;

				// Clear old entry
				tx.clear(entry.key());

				// Read old cpu
				let remaining_cpu_key =
					keys::client::RemainingCpuKey::new(old_allocation_key.client_id);
				let remaining_cpu_key_buf = keys::subspace().pack(&remaining_cpu_key);
				let remaining_cpu_entry = tx.get(&remaining_cpu_key_buf, SERIALIZABLE).await?;
				let old_remaining_cpu = remaining_cpu_key
					.deserialize(
						&remaining_cpu_entry.ok_or(fdb::FdbBindingError::CustomError(
							format!("key should exist: {remaining_cpu_key:?}").into(),
						))?,
					)
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Update allocated amount
				let new_remaining_mem = old_allocation_key.remaining_mem - memory_mib;
				let new_remaining_cpu = old_remaining_cpu - input.resources.cpu;
				let new_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
					client_flavor,
					new_remaining_mem,
					old_allocation_key.last_ping_ts,
					old_allocation_key.client_id,
				);
				tx.set(&keys::subspace().pack(&new_allocation_key), entry.value());

				tracing::debug!(
					old_mem=%old_allocation_key.remaining_mem,
					old_cpu=%old_remaining_cpu,
					new_mem=%new_remaining_mem,
					new_cpu=%new_remaining_cpu,
					"allocating resources"
				);

				// Update client record
				let remaining_mem_key =
					keys::client::RemainingMemoryKey::new(old_allocation_key.client_id);
				tx.set(
					&keys::subspace().pack(&remaining_mem_key),
					&remaining_mem_key
						.serialize(new_remaining_mem)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				tx.set(
					&remaining_cpu_key_buf,
					&remaining_cpu_key
						.serialize(new_remaining_cpu)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				// Insert actor index key
				let client_actor_key =
					keys::client::ActorKey::new(old_allocation_key.client_id, input.actor_id);
				tx.set(
					&keys::subspace().pack(&client_actor_key),
					&client_actor_key
						.serialize(input.generation)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				return Ok(Some(AllocateActorOutputV2 {
					client_id: old_allocation_key.client_id,
					client_workflow_id,
				}));
			}
		})
		.custom_instrument(tracing::info_span!("actor_allocate_tx"))
		.await?;

	let dt = start_instant.elapsed().as_secs_f64();
	metrics::ACTOR_ALLOCATE_DURATION
		.with_label_values(&[&res.is_some().to_string()])
		.observe(dt);

	Ok(res)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct UpdateFdbInput {
	pub actor_id: Uuid,
	pub client_id: Uuid,
	pub state: protocol::ActorState,
}

#[activity(UpdateFdb)]
pub async fn update_fdb(ctx: &ActivityCtx, input: &UpdateFdbInput) -> GlobalResult<()> {
	use protocol::ActorState::*;

	match &input.state {
		Starting | Running { .. } | Stopping => {}
		Stopped | Lost | Exited { .. } => {
			ctx.fdb()
				.await?
				.run(|tx, _mc| async move {
					// Was inserted when the actor was allocated. This is cleared when the state changes as
					// well as when the actor is destroyed to ensure consistency during rescheduling and
					// forced deletion.
					let actor_key = keys::client::ActorKey::new(input.client_id, input.actor_id);
					tx.clear(&keys::subspace().pack(&actor_key));

					Ok(())
				})
				.custom_instrument(tracing::info_span!("actor_clear_tx"))
				.await?;
		}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct UpdateImageInput {
	pub image_id: Uuid,
}

#[activity(UpdateImage)]
pub async fn update_image(ctx: &ActivityCtx, input: &UpdateImageInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET image_id = ?
		",
		input.image_id,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetStartedInput {}

#[activity(SetStarted)]
pub async fn set_started(ctx: &ActivityCtx, input: &SetStartedInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;
	let start_ts = util::timestamp::now();

	let row = sql_fetch_optional!(
		[ctx, (i64,), pool]
		"
		UPDATE state
		SET start_ts = ?
		WHERE start_ts IS NULL
		RETURNING create_ts
		",
		start_ts,
	)
	.await?;

	// Add start duration if this is the first start
	if let Some((create_ts,)) = row {
		let dt = (start_ts - create_ts) as f64 / 1000.0;
		metrics::ACTOR_START_DURATION
			.with_label_values(&[])
			.observe(dt);
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetConnectableInput {
	pub connectable: bool,
}

#[activity(SetConnectable)]
pub async fn set_connectable(ctx: &ActivityCtx, input: &SetConnectableInput) -> GlobalResult<bool> {
	let pool = ctx.sqlite().await?;

	let res = sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET connectable_ts = ?
		WHERE
			CASE WHEN ?
			THEN connectable_ts IS NULL
			ELSE connectable_ts IS NOT NULL
			END
		",
		input.connectable.then(util::timestamp::now),
		input.connectable,
	)
	.await?;

	Ok(res.rows_affected() > 0)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InsertPortsInput {
	pub ports: util::serde::HashableMap<String, protocol::ProxiedPort>,
}

#[activity(InsertPorts)]
pub async fn insert_ports(ctx: &ActivityCtx, input: &InsertPortsInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;
	let mut conn = pool.conn().await?;
	let mut tx = conn.begin().await?;

	for (port_name, port) in &input.ports {
		sql_execute!(
			[ctx, @tx &mut tx]
			"
			INSERT INTO ports_proxied (
				port_name,
				source,
				ip
			)
			VALUES (?, ?, ?)
			",
			port_name,
			port.source as i64,
			&port.lan_hostname,
		)
		.await?;
	}

	tx.commit().await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InsertPortsFdbInput {
	pub actor_id: Uuid,
	pub ports: util::serde::HashableMap<String, protocol::ProxiedPort>,
}

#[activity(InsertPortsFdb)]
pub async fn insert_ports_fdb(ctx: &ActivityCtx, input: &InsertPortsFdbInput) -> GlobalResult<()> {
	let pool = &ctx.sqlite().await?;

	let ((create_ts,), ingress_ports) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (i64,), pool]
			"
			SELECT create_ts
			FROM state 
			",
		),
		sql_fetch_all!(
			[ctx, (String, i64, i64), pool]
			"
			SELECT port_name, ingress_port_number, protocol
			FROM ports_ingress 
			",
		),
	)?;

	let proxied_ports = input
		.ports
		.iter()
		// Match to ingress ports for GG
		.filter_map(|(port_name, port)| {
			if let Some((_, ingress_port_number, protocol)) = ingress_ports
				.iter()
				.find(|(ingress_port_name, _, _)| port_name == ingress_port_name)
			{
				Some((port_name, port, ingress_port_number, protocol))
			} else {
				None
			}
		})
		.map(|(port_name, port, ingress_port_number, protocol)| {
			let protocol = unwrap!(GameGuardProtocol::from_repr((*protocol).try_into()?));

			Ok(keys::actor::ProxiedPort {
				port_name: port_name.clone(),
				create_ts,
				lan_hostname: port.lan_hostname.clone(),
				source: port.source,
				ingress_port_number: (*ingress_port_number).try_into()?,
				protocol,
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Write proxied ingress ports to fdb index
	ctx.fdb()
		.await?
		.run(|tx, _mc| {
			let proxied_ports = proxied_ports.clone();
			async move {
				let proxied_ports_key = keys::actor::ProxiedPortsKey::new(input.actor_id);

				tx.set(
					&keys::subspace().pack(&proxied_ports_key),
					&proxied_ports_key
						.serialize(proxied_ports)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("actor_insert_proxied_ports_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CompareRetryInput {
	last_retry_ts: i64,
}

#[activity(CompareRetry)]
async fn compare_retry(ctx: &ActivityCtx, input: &CompareRetryInput) -> GlobalResult<(i64, bool)> {
	let now = util::timestamp::now();

	// If the last retry ts is more than RETRY_RESET_DURATION_MS, reset retry count
	Ok((now, input.last_retry_ts < now - RETRY_RESET_DURATION_MS))
}

/// Returns whether or not there was availability to spawn the actor.
pub async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	actor_setup: &setup::ActorSetupCtx,
	generation: u32,
) -> GlobalResult<Option<AllocateActorOutputV2>> {
	let res = match ctx.check_version(2).await? {
		1 => {
			ctx.activity(AllocateActorInputV1 {
				actor_id: input.actor_id,
				build_kind: actor_setup.meta.build_kind,
				resources: actor_setup.resources.clone(),
			})
			.await?
		}
		_ => {
			ctx.v(2)
				.activity(AllocateActorInputV2 {
					actor_id: input.actor_id,
					generation,
					build_kind: actor_setup.meta.build_kind,
					resources: actor_setup.resources.clone(),
				})
				.await?
		}
	};

	let Some(res) = res else {
		return Ok(None);
	};

	let (_, ports_res) = ctx
		.join((
			activity(UpdateClientInput {
				client_id: res.client_id,
				client_workflow_id: res.client_workflow_id,
			}),
			v(2).activity(FetchPortsInput {
				actor_id: input.actor_id,
				endpoint_type: input.endpoint_type,
			}),
		))
		.await?;

	let cluster_id = ctx.config().server()?.rivet.edge()?.cluster_id;

	ctx.signal(protocol::Command::StartActor {
		actor_id: input.actor_id,
		generation,
		config: Box::new(protocol::ActorConfig {
			image: protocol::Image {
				id: actor_setup.image_id,
				artifact_url_stub: actor_setup.artifact_url_stub.clone(),
				fallback_artifact_url: actor_setup.fallback_artifact_url.clone(),
				artifact_size_bytes: actor_setup.artifact_size_bytes,
				kind: actor_setup.meta.build_kind.into(),
				compression: actor_setup.meta.build_compression.into(),
			},
			root_user_enabled: input.root_user_enabled,
			env: input.environment.clone(),
			ports: ports_res
				.ports
				.iter()
				.map(|port| match port.port.routing {
					Routing::GameGuard { protocol } => (
						crate::util::pegboard_normalize_port_name(&port.name),
						protocol::Port {
							target: port.port_number,
							protocol: match protocol {
								GameGuardProtocol::Http
								| GameGuardProtocol::Https
								| GameGuardProtocol::Tcp
								| GameGuardProtocol::TcpTls => protocol::TransportProtocol::Tcp,
								GameGuardProtocol::Udp => protocol::TransportProtocol::Udp,
							},
							routing: protocol::PortRouting::GameGuard,
						},
					),
					Routing::Host { protocol } => (
						crate::util::pegboard_normalize_port_name(&port.name),
						protocol::Port {
							target: port.port_number,
							protocol: match protocol {
								HostProtocol::Tcp => protocol::TransportProtocol::Tcp,
								HostProtocol::Udp => protocol::TransportProtocol::Udp,
							},
							routing: protocol::PortRouting::Host,
						},
					),
				})
				.collect(),
			network_mode: match input.network_mode {
				NetworkMode::Bridge => protocol::NetworkMode::Bridge,
				NetworkMode::Host => protocol::NetworkMode::Host,
			},
			resources: actor_setup.resources.clone(),
			metadata: util::serde::Raw::new(&protocol::ActorMetadata {
				actor: protocol::ActorMetadataActor {
					actor_id: input.actor_id,
					tags: input.tags.clone(),
					create_ts: ctx.ts(),
				},
				network: Some(protocol::ActorMetadataNetwork {
					ports: ports_res
						.ports
						.into_iter()
						.map(|port| (port.name, port.port))
						.collect(),
				}),
				project: protocol::ActorMetadataProject {
					project_id: actor_setup.meta.project_id,
					slug: actor_setup.meta.project_slug.clone(),
				},
				environment: protocol::ActorMetadataEnvironment {
					env_id: input.env_id,
					slug: actor_setup.meta.env_slug.clone(),
				},
				datacenter: protocol::ActorMetadataDatacenter {
					name_id: actor_setup.meta.dc_name_id.clone(),
					display_name: actor_setup.meta.dc_display_name.clone(),
				},
				cluster: protocol::ActorMetadataCluster { cluster_id },
				build: protocol::ActorMetadataBuild {
					build_id: input.image_id,
				},
			})?,
		}),
	})
	.to_workflow_id(res.client_workflow_id)
	.send()
	.await?;

	Ok(Some(res))
}

pub async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut State,
	image_id: Uuid,
) -> GlobalResult<Option<Destroy>> {
	tracing::debug!(actor_id=?input.actor_id, "rescheduling actor");

	ctx.activity(ClearPortsAndResourcesInput {
		actor_id: input.actor_id,
		image_id,
		client_id: state.client_id,
		client_workflow_id: state.client_workflow_id,
	})
	.await?;

	let actor_setup = setup::setup(ctx, &input, setup::SetupCtx::Reschedule { image_id }).await?;

	let next_generation = state.generation + 1;

	// Waits for the actor to be ready (or destroyed) and automatically retries if failed to allocate.
	let res = ctx
		.loope(state.reschedule_state.clone(), |ctx, state| {
			let input = input.clone();
			let actor_setup = actor_setup.clone();

			async move {
				// Determine next backoff sleep duration
				let mut backoff =
					util::Backoff::new_at(8, None, BASE_RETRY_TIMEOUT_MS, 500, state.retry_count);

				let (now, reset) = ctx
					.v(2)
					.activity(CompareRetryInput {
						last_retry_ts: state.last_retry_ts,
					})
					.await?;

				state.retry_count = if reset { 0 } else { state.retry_count + 1 };
				state.last_retry_ts = now;

				// Don't sleep for first retry
				if state.retry_count > 0 {
					let next = backoff.step().expect("should not have max retry");

					// Sleep for backoff or destroy early
					if let Some(sig) = ctx
						.listen_with_timeout::<Destroy>(Instant::from(next) - Instant::now())
						.await?
					{
						tracing::debug!("destroying before actor reschedule");

						return Ok(Loop::Break(Err(sig)));
					}
				}

				if let Some(res) = spawn_actor(ctx, &input, &actor_setup, next_generation).await? {
					Ok(Loop::Break(Ok((state.clone(), res))))
				} else {
					tracing::debug!(actor_id=?input.actor_id, "failed to reschedule actor, retrying");

					Ok(Loop::Continue)
				}
			}
			.boxed()
		})
		.await?;

	// Update loop state
	match res {
		Ok((reschedule_state, res)) => {
			state.generation = next_generation;
			state.client_id = res.client_id;
			state.client_workflow_id = res.client_workflow_id;

			// Save reschedule state in global state
			state.reschedule_state = reschedule_state;

			// Reset gc timeout once allocated
			state.gc_timeout_ts = Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS);

			Ok(None)
		}
		Err(sig) => Ok(Some(sig)),
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearPortsAndResourcesInput {
	actor_id: Uuid,
	image_id: Uuid,
	client_id: Uuid,
	client_workflow_id: Uuid,
}

#[activity(ClearPortsAndResources)]
async fn clear_ports_and_resources(
	ctx: &ActivityCtx,
	input: &ClearPortsAndResourcesInput,
) -> GlobalResult<()> {
	let pool = &ctx.sqlite().await?;

	let (
		build_res,
		ingress_ports,
		(selected_resources_cpu_millicores, selected_resources_memory_mib),
		_,
	) = tokio::try_join!(
		ctx.op(build::ops::get::Input {
			build_ids: vec![input.image_id],
		}),
		sql_fetch_all!(
			[ctx, (i64, i64), pool]
			"
			SELECT protocol, ingress_port_number
			FROM ports_ingress
			",
		),
		sql_fetch_one!(
			[ctx, (Option<i64>, Option<i64>), pool]
			"
			SELECT selected_resources_cpu_millicores, selected_resources_memory_mib
			FROM state
			",
		),
		// Idempotent
		sql_execute!(
			[ctx, pool]
			"
			DELETE FROM ports_proxied
			",
		),
	)?;
	let build = unwrap_with!(build_res.builds.first(), BUILD_NOT_FOUND);

	ctx.fdb()
		.await?
		.run(|tx, _mc| {
			let ingress_ports = ingress_ports.clone();
			async move {
				destroy::clear_ports_and_resources(
					input.actor_id,
					Some(build.kind),
					ingress_ports,
					Some(input.client_id),
					Some(input.client_workflow_id),
					selected_resources_memory_mib,
					selected_resources_cpu_millicores,
					&tx,
				)
				.await
			}
		})
		.custom_instrument(tracing::info_span!("actor_clear_ports_and_resources_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetFinishedInput {}

#[activity(SetFinished)]
pub async fn set_finished(ctx: &ActivityCtx, input: &SetFinishedInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET finish_ts = ?
		",
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}
