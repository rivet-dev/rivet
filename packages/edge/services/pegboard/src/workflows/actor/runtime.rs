use std::time::Instant;

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use fdb_util::{end_of_key_range, FormalKey, SERIALIZABLE, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
};
use futures_util::{FutureExt, TryStreamExt};
use sqlx::Acquire;
use util::serde::AsHashableExt;

use super::{
	destroy::KillCtx, setup, Destroy, Input, Port, ACTOR_START_THRESHOLD_MS, BASE_RETRY_TIMEOUT_MS,
};
use crate::{
	keys, metrics, protocol,
	types::{GameGuardProtocol, HostProtocol, NetworkMode, Routing},
	workflows::client::CLIENT_ELIGIBLE_THRESHOLD_MS,
};

#[derive(Deserialize, Serialize)]
pub struct State {
	pub client_id: Uuid,
	pub drain_timeout_ts: Option<i64>,
	pub gc_timeout_ts: Option<i64>,
}

impl State {
	pub fn new(client_id: Uuid) -> Self {
		State {
			client_id,
			drain_timeout_ts: None,
			gc_timeout_ts: Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS),
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct StateRes {
	pub kill: Option<KillCtx>,
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
		client_wan_hostname,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct AllocateActorInput {
	actor_id: Uuid,
	build_kind: BuildKind,
	resources: protocol::Resources,
}

#[derive(Debug, Serialize, Deserialize)]
struct AllocateActorOutput {
	client_id: Uuid,
	client_workflow_id: Uuid,
}

#[activity(AllocateActor)]
async fn allocate_actor(
	ctx: &ActivityCtx,
	input: &AllocateActorInput,
) -> GlobalResult<Option<AllocateActorOutput>> {
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
						.serialize(())
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				return Ok(Some(AllocateActorOutput {
					client_id: old_allocation_key.client_id,
					client_workflow_id,
				}));
			}
		})
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

	sql_execute!(
		[ctx, pool]
		"
		UPDATE state
		SET start_ts = ?
		WHERE start_ts IS NULL
		",
		util::timestamp::now(),
	)
	.await?;

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
		.await?;

	Ok(())
}

/// Returns whether or not there was availability to spawn the actor.
pub async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	network_ports: &util::serde::HashableMap<String, Port>,
	actor_setup: &setup::ActorSetupCtx,
) -> GlobalResult<Option<Uuid>> {
	let Some(res) = ctx
		.activity(AllocateActorInput {
			actor_id: input.actor_id,
			build_kind: actor_setup.meta.build_kind,
			resources: actor_setup.resources.clone(),
		})
		.await?
	else {
		return Ok(None);
	};

	ctx.activity(UpdateClientInput {
		client_id: res.client_id,
		client_workflow_id: res.client_workflow_id,
	})
	.await?;

	let cluster_id = ctx.config().server()?.rivet.edge()?.cluster_id;

	ctx.signal(protocol::Command::StartActor {
		actor_id: input.actor_id,
		config: Box::new(protocol::ActorConfig {
			image: protocol::Image {
				id: input.image_id,
				artifact_url_stub: actor_setup.artifact_url_stub.clone(),
				fallback_artifact_url: actor_setup.fallback_artifact_url.clone(),
				kind: match actor_setup.meta.build_kind {
					BuildKind::DockerImage => protocol::ImageKind::DockerImage,
					BuildKind::OciBundle => protocol::ImageKind::OciBundle,
					BuildKind::JavaScript => protocol::ImageKind::JavaScript,
				},
				compression: match actor_setup.meta.build_compression {
					BuildCompression::None => protocol::ImageCompression::None,
					BuildCompression::Lz4 => protocol::ImageCompression::Lz4,
				},
			},
			root_user_enabled: input.root_user_enabled,
			env: input.environment.as_hashable(),
			ports: network_ports
				.iter()
				.map(|(port_name, port)| match port.routing {
					Routing::GameGuard { protocol, .. } => (
						crate::util::pegboard_normalize_port_name(port_name),
						protocol::Port {
							target: port.internal_port,
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
						crate::util::pegboard_normalize_port_name(port_name),
						protocol::Port {
							target: port.internal_port,
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
					tags: input.tags.as_hashable(),
					create_ts: ctx.ts(),
				},
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

	Ok(Some(res.client_id))
}

pub async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	network_ports: &util::serde::HashableMap<String, Port>,
	state: &mut State,
	new_image_id: Option<Uuid>,
) -> GlobalResult<Option<Destroy>> {
	tracing::info!("rescheduling actor");

	// Remove old proxied ports
	ctx.activity(ClearPortsInput {
		actor_id: input.actor_id,
	})
	.await?;

	let actor_setup = setup::setup(
		ctx,
		&input,
		&network_ports,
		setup::SetupCtx::Reschedule { new_image_id },
	)
	.await?;

	// Waits for the actor to be ready (or destroyed) and automatically retries if failed to allocate.
	let res = ctx
		.loope(RescheduleState::default(), |ctx, state| {
			let input = input.clone();
			let actor_setup = actor_setup.clone();
			let network_ports = network_ports.clone();

			async move {
				// Determine next backoff sleep duration
				let mut backoff =
					util::Backoff::new_at(8, None, BASE_RETRY_TIMEOUT_MS, 500, state.retry_count);

				// If the last retry ts is more than 2 * backoff ago, reset retry count to 0
				let now = util::timestamp::now();
				state.retry_count =
					if state.last_retry_ts < now - i64::try_from(2 * backoff.current_duration())? {
						state.retry_count + 1
					} else {
						0
					};
				state.last_retry_ts = now;

				// Don't sleep for first retry
				if state.retry_count > 0 {
					let next = backoff.step().expect("should not have max retry");

					// Sleep for backoff or destroy early
					if let Some(sig) = ctx
						.listen_with_timeout::<Destroy>(Instant::from(next) - Instant::now())
						.await?
					{
						tracing::debug!("destroying before actor start");

						return Ok(Loop::Break(Err(sig)));
					}
				}

				if let Some(client_id) =
					spawn_actor(ctx, &input, &network_ports, &actor_setup).await?
				{
					Ok(Loop::Break(Ok(client_id)))
				} else {
					Ok(Loop::Continue)
				}
			}
			.boxed()
		})
		.await?;

	match res {
		Ok(client_id) => {
			state.client_id = client_id;
			Ok(None)
		}
		Err(sig) => Ok(Some(sig)),
	}
}

#[derive(Serialize, Deserialize, Default)]
struct RescheduleState {
	last_retry_ts: i64,
	retry_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearPortsInput {
	actor_id: Uuid,
}

#[activity(ClearPorts)]
async fn clear_ports(ctx: &ActivityCtx, input: &ClearPortsInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	sql_execute!(
		[ctx, pool]
		"
		DELETE FROM ports_proxied
		",
	)
	.await?;

	// It is ok for both of these to be in the same activity because they are idempotent. There cannot be any
	// ports inserted if this activity is running because the insertion of ports happens in the same workflow.
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let proxied_ports_key = keys::actor::ProxiedPortsKey::new(input.actor_id);
			tx.clear(&keys::subspace().pack(&proxied_ports_key));

			Ok(())
		})
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
