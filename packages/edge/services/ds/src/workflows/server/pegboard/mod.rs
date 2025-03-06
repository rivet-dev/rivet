use std::{
	collections::{HashMap, HashSet},
	time::Duration,
};

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use destroy::DestroyActor;
use fdb_util::FormalKey;
use foundationdb as fdb;
use futures_util::FutureExt;
use pegboard::protocol as pp;
use serde_json::json;
use sqlx::Acquire;
use tokio::time::Instant;
use util::serde::AsHashableExt;

use super::{
	CreateComplete, Destroy, Failed, GetServerMetaInput, GetServerMetaOutput, InsertDbInput,
	PopulateFdbIdxInput, Port, Ready, SetConnectableInput, UpdateImageInput, Upgrade,
	UpgradeComplete, UpgradeStarted, BASE_RETRY_TIMEOUT_MS, DRAIN_PADDING_MS,
};
use crate::{
	keys,
	types::{
		GameGuardProtocol, HostProtocol, NetworkMode, Routing, ServerLifecycle, ServerResources,
	},
};

pub mod destroy;
mod migrations;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub lifecycle: ServerLifecycle,
	pub image_id: Uuid,
	pub root_user_enabled: bool,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[workflow]
pub(crate) async fn ds_server_pegboard(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let res = setup(ctx, input, SetupCtx::Init).await;
	let initial_actor_setup = match ctx.catch_unrecoverable(res)? {
		Ok(res) => res,
		Err(err) => {
			tracing::error!(?err, "unrecoverable setup");

			ctx.msg(Failed {
				message: "Failed setup.".into(),
			})
			.tag("server_id", input.server_id)
			.send()
			.await?;

			ctx.workflow(destroy::Input {
				server_id: input.server_id,
				destroy_actor: None,
			})
			.output()
			.await?;

			// Throw the original error from the setup activities
			return Err(err);
		}
	};

	ctx.msg(CreateComplete {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	let Some(client_id) = spawn_actor(ctx, input, &initial_actor_setup).await? else {
		ctx.msg(Failed {
			message: "Failed to allocate (no availability).".into(),
		})
		.tag("server_id", input.server_id)
		.send()
		.await?;

		ctx.workflow(destroy::Input {
			server_id: input.server_id,
			destroy_actor: None,
		})
		.output()
		.await?;

		return Ok(());
	};

	let state_res = ctx
		.loope(
			State::new(initial_actor_setup.actor_id, client_id),
			|ctx, state| {
				let input = input.clone();

				async move {
					let sig = if let Some(drain_timeout_ts) = state.drain_timeout_ts {
						// Listen for signal with drain timeout
						if let Some(sig) = ctx.listen_until::<Main>(drain_timeout_ts).await? {
							sig
						}
						// Reschedule durable actor on drain end
						else if input.lifecycle.durable {
							ctx.activity(SetConnectableInput { connectable: false })
								.await?;

							// Kill old actor immediately
							destroy::destroy_actor(ctx, state.actor_id, state.client_id, 0, true)
								.await?;

							if let Some(sig) = reschedule_actor(ctx, &input, state, None).await? {
								// Destroyed early
								return Ok(Loop::Break(StateRes {
									destroy_actor: Some(DestroyActor {
										actor_id: state.actor_id,
										client_id: state.client_id,
										kill_timeout_ms: sig
											.override_kill_timeout_ms
											.unwrap_or(input.lifecycle.kill_timeout_ms),
									}),
								}));
							} else {
								state.drain_timeout_ts = None;
								return Ok(Loop::Continue);
							}
						} else {
							return Ok(Loop::Break(StateRes {
								destroy_actor: Some(DestroyActor {
									actor_id: state.actor_id,
									client_id: state.client_id,
									kill_timeout_ms: input.lifecycle.kill_timeout_ms,
								}),
							}));
						}
					} else {
						// Listen for signal normally
						ctx.listen::<Main>().await?
					};

					match sig {
						Main::ActorStateUpdate(sig) => match sig.state {
							pp::ActorState::Starting => {
								ctx.activity(SetStartedInput {}).await?;
							}
							pp::ActorState::Running { ports, .. } => {
								ctx.activity(InsertPortsInput {
									ports: ports.clone(),
								})
								.await?;
								ctx.activity(InsertPortsFdbInput {
									server_id: input.server_id,
									ports,
								})
								.await?;

								// Wait for Traefik to poll ports and update GG
								ctx.activity(WaitForTraefikPollInput {
									create_ts: ctx.ts(),
								})
								.await?;

								let updated = ctx
									.activity(SetConnectableInput { connectable: true })
									.await?;

								if updated {
									ctx.msg(Ready {})
										.tag("server_id", input.server_id)
										.send()
										.await?;
								}
							}
							pp::ActorState::Stopping | pp::ActorState::Stopped => {}
							pp::ActorState::Exited { .. } | pp::ActorState::Lost => {
								let exit_code =
									if let pp::ActorState::Exited { exit_code } = sig.state {
										exit_code
									} else {
										None
									};

								tracing::debug!(?exit_code, "actor stopped");

								let failed =
									exit_code.map(|exit_code| exit_code != 0).unwrap_or(true);

								// Reschedule durable actor if it errored
								if input.lifecycle.durable && failed {
									ctx.activity(SetConnectableInput { connectable: false })
										.await?;

									if reschedule_actor(ctx, &input, state, None).await?.is_some() {
										// Destroyed early
										return Ok(Loop::Break(StateRes {
											// Destroy actor is none here because if we received the destroy
											// signal, it is guaranteed that we did not allocate another actor.
											destroy_actor: None,
										}));
									}
								} else {
									ctx.activity(SetFinishedInput {}).await?;

									return Ok(Loop::Break(StateRes {
										destroy_actor: None,
									}));
								}
							}
							pp::ActorState::Draining { drain_timeout_ts } => {
								state.drain_timeout_ts = Some(
									drain_timeout_ts
										- DRAIN_PADDING_MS - input.lifecycle.kill_timeout_ms,
								);
							}
							pp::ActorState::Undrained => {
								state.drain_timeout_ts = None;
							}
						},
						Main::Upgrade(sig) => {
							ctx.msg(UpgradeStarted {})
								.tag("server_id", input.server_id)
								.send()
								.await?;

							ctx.activity(SetConnectableInput { connectable: false })
								.await?;

							// Kill old actor immediately
							destroy::destroy_actor(ctx, state.actor_id, state.client_id, 0, true)
								.await?;

							if let Some(sig) =
								reschedule_actor(ctx, &input, state, Some(sig.image_id)).await?
							{
								// Destroyed early
								return Ok(Loop::Break(StateRes {
									destroy_actor: Some(DestroyActor {
										actor_id: state.actor_id,
										client_id: state.client_id,
										kill_timeout_ms: sig
											.override_kill_timeout_ms
											.unwrap_or(input.lifecycle.kill_timeout_ms),
									}),
								}));
							}

							ctx.msg(UpgradeComplete {})
								.tag("server_id", input.server_id)
								.send()
								.await?;
						}
						Main::Destroy(sig) => {
							return Ok(Loop::Break(StateRes {
								destroy_actor: Some(DestroyActor {
									actor_id: state.actor_id,
									client_id: state.client_id,
									kill_timeout_ms: sig
										.override_kill_timeout_ms
										.unwrap_or(input.lifecycle.kill_timeout_ms),
								}),
							}))
						}
					}

					Ok(Loop::Continue)
				}
				.boxed()
			},
		)
		.await?;

	ctx.workflow(destroy::Input {
		server_id: input.server_id,
		destroy_actor: state_res.destroy_actor,
	})
	.output()
	.await?;

	Ok(())
}

#[derive(Deserialize, Serialize)]
struct State {
	actor_id: Uuid,
	client_id: Uuid,
	drain_timeout_ts: Option<i64>,
}

impl State {
	fn new(actor_id: Uuid, client_id: Uuid) -> Self {
		State {
			actor_id,
			client_id,
			drain_timeout_ts: None,
		}
	}
}

#[derive(Serialize, Deserialize)]
struct StateRes {
	destroy_actor: Option<DestroyActor>,
}

enum SetupCtx {
	Init,
	Reschedule { new_image_id: Option<Uuid> },
}

#[derive(Clone)]
struct ActorSetupCtx {
	actor_id: Uuid,
	server_meta: GetServerMetaOutput,
	resources: pp::Resources,
	artifact_url_stub: String,
	fallback_artifact_url: Option<String>,
}

async fn setup(
	ctx: &mut WorkflowCtx,
	input: &Input,
	setup: SetupCtx,
) -> GlobalResult<ActorSetupCtx> {
	migrations::run(ctx).await?;

	let image_id = match &setup {
		SetupCtx::Init => {
			let tags = input.tags.as_hashable();
			let create_ts = ctx
				.activity(InsertDbInput {
					server_id: input.server_id,
					env_id: input.env_id,
					tags: tags.clone(),
					resources: input.resources.clone(),
					lifecycle: input.lifecycle.clone(),
					image_id: input.image_id,
					args: input.args.clone(),
					network_mode: input.network_mode,
					environment: input.environment.as_hashable(),
					network_ports: input.network_ports.as_hashable(),
				})
				.await?;

			ctx.activity(PopulateFdbIdxInput {
				server_id: input.server_id,
				env_id: input.env_id,
				tags,
				create_ts,
			})
			.await?;

			input.image_id
		}
		SetupCtx::Reschedule { new_image_id } => {
			if let Some(image_id) = *new_image_id {
				ctx.activity(UpdateImageInput { image_id }).await?;

				image_id
			} else {
				input.image_id
			}
		}
	};

	let server_meta = ctx
		.activity(GetServerMetaInput {
			env_id: input.env_id,
			image_id,
		})
		.await?;

	let (actor_id, resources, artifacts_res) = ctx
		.join((
			activity(SelectActorIdInput {}),
			activity(SelectResourcesInput {
				resources: input.resources.clone(),
			}),
			activity(ResolveArtifactsInput {
				build_upload_id: server_meta.build_upload_id,
				build_file_name: server_meta.build_file_name.clone(),
				dc_build_delivery_method: server_meta.dc_build_delivery_method,
			}),
		))
		.await?;

	Ok(ActorSetupCtx {
		actor_id,
		server_meta,
		resources,
		artifact_url_stub: artifacts_res.artifact_url_stub,
		fallback_artifact_url: artifacts_res.fallback_artifact_url,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SelectActorIdInput {}

#[activity(SelectActorId)]
async fn select_actor_id(ctx: &ActivityCtx, input: &SelectActorIdInput) -> GlobalResult<Uuid> {
	Ok(Uuid::new_v4())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct AllocateActorInput {
	actor_id: Uuid,
	build_kind: BuildKind,
	resources: pp::Resources,
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
		BuildKind::DockerImage | BuildKind::OciBundle => pp::ClientFlavor::Container,
		BuildKind::JavaScript => pp::ClientFlavor::Isolate,
	};
	let memory_mib = input.resources.memory / 1024 / 1024;

	let res = ctx
		.op(pegboard::ops::client::reserve::Input {
			flavor: client_flavor,
			memory: memory_mib,
			cpu: input.resources.cpu,
		})
		.await?;

	Ok(res.map(|res| AllocateActorOutput {
		client_id: res.client_id,
		client_workflow_id: res.client_workflow_id,
	}))
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
		UPDATE pegboard
		SET
			client_id = ?,
			client_wan_hostname = ?
		",
		input.client_id,
		client_wan_hostname,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SelectResourcesInput {
	resources: ServerResources,
}

#[activity(SelectResources)]
async fn select_resources(
	ctx: &ActivityCtx,
	input: &SelectResourcesInput,
) -> GlobalResult<pp::Resources> {
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	let tier_res = ctx
		.op(tier::ops::list::Input {
			datacenter_ids: vec![dc_id],
			pegboard: true,
		})
		.await?;
	let tier_dc = unwrap!(tier_res.datacenters.first());
	let mut tiers = tier_dc.tiers.iter().collect::<Vec<_>>();

	// Sort the tiers by cpu
	tiers.sort_by(|a, b| a.cpu.cmp(&b.cpu));

	// Find the first tier that has more CPU and memory than the requested
	// resources
	let tier = unwrap!(tiers.iter().find(|t| {
		t.cpu_millicores >= input.resources.cpu_millicores && t.memory >= input.resources.memory_mib
	}));

	// runc-compatible resources
	let cpu = tier.rivet_cores_numerator as u64 * 1_000 / tier.rivet_cores_denominator as u64; // Millicore (1/1000 of a core)
	let memory = tier.memory as u64 * (1024 * 1024);
	let memory_max = tier.memory_max as u64 * (1024 * 1024);

	Ok(pp::Resources {
		cpu,
		memory,
		memory_max,
		disk: tier.disk,
	})
}

/// Returns whether or not there was availability to spawn the actor.
async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	actor_setup: &ActorSetupCtx,
) -> GlobalResult<Option<Uuid>> {
	let Some(res) = ctx
		.activity(AllocateActorInput {
			actor_id: actor_setup.actor_id,
			build_kind: actor_setup.server_meta.build_kind,
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

	ctx.signal(pp::Command::StartActor {
		actor_id: actor_setup.actor_id,
		config: Box::new(pp::ActorConfig {
			image: pp::Image {
				id: input.image_id,
				artifact_url_stub: actor_setup.artifact_url_stub.clone(),
				fallback_artifact_url: actor_setup.fallback_artifact_url.clone(),
				kind: match actor_setup.server_meta.build_kind {
					BuildKind::DockerImage => pp::ImageKind::DockerImage,
					BuildKind::OciBundle => pp::ImageKind::OciBundle,
					BuildKind::JavaScript => pp::ImageKind::JavaScript,
				},
				compression: match actor_setup.server_meta.build_compression {
					BuildCompression::None => pp::ImageCompression::None,
					BuildCompression::Lz4 => pp::ImageCompression::Lz4,
				},
			},
			root_user_enabled: input.root_user_enabled,
			env: input.environment.as_hashable(),
			ports: input
				.network_ports
				.iter()
				.map(|(port_name, port)| match port.routing {
					Routing::GameGuard { protocol, .. } => (
						crate::util::pegboard_normalize_port_name(port_name),
						pp::Port {
							target: port.internal_port,
							protocol: match protocol {
								GameGuardProtocol::Http
								| GameGuardProtocol::Https
								| GameGuardProtocol::Tcp
								| GameGuardProtocol::TcpTls => pp::TransportProtocol::Tcp,
								GameGuardProtocol::Udp => pp::TransportProtocol::Udp,
							},
							routing: pp::PortRouting::GameGuard,
						},
					),
					Routing::Host { protocol } => (
						crate::util::pegboard_normalize_port_name(port_name),
						pp::Port {
							target: port.internal_port,
							protocol: match protocol {
								HostProtocol::Tcp => pp::TransportProtocol::Tcp,
								HostProtocol::Udp => pp::TransportProtocol::Udp,
							},
							routing: pp::PortRouting::Host,
						},
					),
				})
				.collect(),
			network_mode: match input.network_mode {
				NetworkMode::Bridge => pp::NetworkMode::Bridge,
				NetworkMode::Host => pp::NetworkMode::Host,
			},
			resources: actor_setup.resources.clone(),
			owner: pp::ActorOwner::DynamicServer {
				server_id: input.server_id,
				workflow_id: ctx.workflow_id(),
			},
			metadata: util::serde::Raw::new(&pp::ActorMetadata {
				actor: pp::ActorMetadataActor {
					actor_id: actor_setup.actor_id,
					tags: input.tags.as_hashable(),
					// Represents when the pegboard actor was created, not the ds workflow.
					create_ts: ctx.ts(),
				},
				project: pp::ActorMetadataProject {
					project_id: actor_setup.server_meta.project_id,
					slug: actor_setup.server_meta.project_slug.clone(),
				},
				environment: pp::ActorMetadataEnvironment {
					env_id: input.env_id,
					slug: actor_setup.server_meta.env_slug.clone(),
				},
				datacenter: pp::ActorMetadataDatacenter {
					name_id: actor_setup.server_meta.dc_name_id.clone(),
					display_name: actor_setup.server_meta.dc_display_name.clone(),
				},
				cluster: pp::ActorMetadataCluster { cluster_id },
				build: pp::ActorMetadataBuild {
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

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsInput {
	build_upload_id: Uuid,
	build_file_name: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsOutput {
	artifact_url_stub: String,
	fallback_artifact_url: Option<String>,
}

#[activity(ResolveArtifacts)]
async fn resolve_artifacts(
	ctx: &ActivityCtx,
	input: &ResolveArtifactsInput,
) -> GlobalResult<ResolveArtifactsOutput> {
	let fallback_artifact_url =
		if let BuildDeliveryMethod::S3Direct = input.dc_build_delivery_method {
			tracing::debug!("using s3 direct delivery");

			// Build client
			let s3_client = s3_util::Client::with_bucket_and_endpoint(
				ctx.config(),
				"bucket-build",
				s3_util::EndpointKind::EdgeInternal,
			)
			.await?;

			let presigned_req = s3_client
				.get_object()
				.bucket(s3_client.bucket())
				.key(format!(
					"{upload_id}/{file_name}",
					upload_id = input.build_upload_id,
					file_name = input.build_file_name,
				))
				.presigned(
					s3_util::aws_sdk_s3::presigning::PresigningConfig::builder()
						.expires_in(std::time::Duration::from_secs(15 * 60))
						.build()?,
				)
				.await?;

			let addr_str = presigned_req.uri().to_string();
			tracing::debug!(addr = %addr_str, "resolved artifact s3 presigned request");

			Some(addr_str)
		} else {
			None
		};

	Ok(ResolveArtifactsOutput {
		artifact_url_stub: crate::util::image_artifact_url_stub(
			ctx.config(),
			input.build_upload_id,
			&input.build_file_name,
		)?,
		fallback_artifact_url,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetStartedInput {}

#[activity(SetStarted)]
async fn set_started(ctx: &ActivityCtx, input: &SetStartedInput) -> GlobalResult<()> {
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
struct InsertPortsInput {
	ports: util::serde::HashableMap<String, pp::ProxiedPort>,
}

#[activity(InsertPorts)]
async fn insert_ports(ctx: &ActivityCtx, input: &InsertPortsInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;
	let mut conn = pool.conn().await?;
	let mut tx = conn.begin().await?;

	for (port_name, port) in &input.ports {
		sql_execute!(
			[ctx, @tx &mut tx]
			"
			INSERT INTO server_ports_proxied (
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
struct InsertPortsFdbInput {
	server_id: Uuid,
	ports: util::serde::HashableMap<String, pp::ProxiedPort>,
}

#[activity(InsertPortsFdb)]
async fn insert_ports_fdb(ctx: &ActivityCtx, input: &InsertPortsFdbInput) -> GlobalResult<()> {
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
			FROM server_ports_ingress 
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

			Ok(keys::server::ProxiedPort {
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
				let proxied_ports_key = keys::server::ProxiedPortsKey::new(input.server_id);

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

async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut State,
	new_image_id: Option<Uuid>,
) -> GlobalResult<Option<Destroy>> {
	tracing::info!("rescheduling actor");

	// Remove old proxied ports
	ctx.activity(ClearPortsInput {
		server_id: input.server_id,
	})
	.await?;

	let actor_setup = setup(ctx, &input, SetupCtx::Reschedule { new_image_id }).await?;
	state.actor_id = actor_setup.actor_id;

	// Waits for the actor to be ready (or destroyed) and automatically retries if failed to allocate.
	let res = ctx
		.loope(RescheduleState::default(), |ctx, state| {
			let input = input.clone();
			let actor_setup = actor_setup.clone();

			async move {
				// Determine next backoff sleep duration
				let mut backoff = rivet_util::Backoff::new_at(
					8,
					None,
					BASE_RETRY_TIMEOUT_MS,
					500,
					state.retry_count,
				);

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
						.listen_with_timeout::<Destroy>(next - Instant::now())
						.await?
					{
						tracing::debug!("destroying before actor start");

						return Ok(Loop::Break(Err(sig)));
					}
				}

				if let Some(client_id) = spawn_actor(ctx, &input, &actor_setup).await? {
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
	server_id: Uuid,
}

#[activity(ClearPorts)]
async fn clear_ports(ctx: &ActivityCtx, input: &ClearPortsInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	sql_execute!(
		[ctx, pool]
		"
		DELETE FROM server_ports_proxied
		",
	)
	.await?;

	// It is ok for both of these to be in the same activity because they are idempotent. There cannot be any
	// ports inserted if this activity is running because the insertion of ports happens in the same workflow.
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let proxied_ports_key = keys::server::ProxiedPortsKey::new(input.server_id);
			tx.clear(&keys::subspace().pack(&proxied_ports_key));

			Ok(())
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetFinishedInput {}

#[activity(SetFinished)]
async fn set_finished(ctx: &ActivityCtx, input: &SetFinishedInput) -> GlobalResult<()> {
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

/// Amount of time to wait after all servers have successfully polled to wait to return in order to
/// avoid a race condition.
///
/// This can likely be decreased to < 100 ms safely.
const TRAEFIK_POLL_COMPLETE_GRACE: Duration = Duration::from_millis(750);

/// Max time to wait for servers to poll their configs.
const TRAEFIK_POLL_TIMEOUT: Duration = Duration::from_secs(5);

/// How logn to wait if no GG servers were returned from the list. This is either from:
/// - Cluster without provisioning configured
/// - Edge case where all GG servers were destroyed and waiting for new servers to come up
const TRAFEIK_NO_SERVERS_GRACE: Duration = Duration::from_millis(500);

#[message("ds_traefik_poll")]
pub struct TraefikPoll {
	/// Server ID will be `None` if:
	/// - Not using provisioning (i.e. self-hosted cluster) or
	/// - Older GG node that's being upgraded
	pub server_id: Option<Uuid>,
	pub latest_ds_create_ts: i64,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct WaitForTraefikPollInput {
	create_ts: i64,
}

/// Waits for all of the GG nodes to poll the Traefik config.
///
/// This is done by waiting for an event to be published for each of the GG server IDs with a
/// timestamp for the latest DS it's seen that's > than this DS's create ts.
#[activity(WaitForTraefikPoll)]
async fn wait_for_traefik_poll(
	ctx: &ActivityCtx,
	input: &WaitForTraefikPollInput,
) -> GlobalResult<()> {
	// TODO: This will only work with 1 node on self-hosted. RG 2 will be out by then which fixes
	// this issue.

	let cluster_id = ctx.config().server()?.rivet.edge()?.cluster_id;
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	// Start sub first since the messages may arrive while fetching the server list
	let mut sub = ctx
		.subscribe::<TraefikPoll>(&json!({ "datacenter_id": dc_id }))
		.await?;

	// Fetch servers
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: cluster::types::Filter {
				pool_types: Some(vec![cluster::types::PoolType::Gg]),
				cluster_ids: Some(vec![cluster_id]),
				..Default::default()
			},
			include_destroyed: false,
			exclude_draining: true,
			exclude_no_vlan: false,
		})
		.await?;

	let mut remaining_servers: HashSet<Uuid> = if servers_res.servers.is_empty() {
		// HACK: Will wait for a single server poll if we don't have the server list. Wait for a
		// static amount of time.
		tokio::time::sleep(TRAFEIK_NO_SERVERS_GRACE).await;
		return Ok(());
	} else {
		servers_res.servers.iter().map(|s| s.server_id).collect()
	};

	tracing::debug!(
		servers=?remaining_servers,
		after_create_ts=?input.create_ts,
		"waiting for traefik servers",
	);
	let res = tokio::time::timeout(TRAEFIK_POLL_TIMEOUT, async {
		// Wait for servers to fetch their configs
		loop {
			let msg = sub.next().await?;

			if let Some(server_id) = msg.server_id {
				if msg.latest_ds_create_ts >= input.create_ts {
					let _did_remove = remaining_servers.remove(&server_id);

					tracing::debug!(
						server_id=?msg.server_id,
						latest_ds_create_ts=?msg.latest_ds_create_ts,
						servers=?remaining_servers, "received poll from traefik server",
					);

					// Break loop once all servers have polled
					if remaining_servers.is_empty() {
						return GlobalResult::Ok(());
					}
				}
			}
		}
	})
	.await;

	match res {
		Ok(_) => {
			tracing::debug!("received poll from all traefik servers, waiting for grace period");
			tokio::time::sleep(TRAEFIK_POLL_COMPLETE_GRACE).await;
		}
		Err(_) => {
			tracing::warn!(missing_server_ids = ?remaining_servers, "did not receive poll from all gg servers before deadline");
		}
	}

	Ok(())
}

join_signal!(Main {
	ActorStateUpdate(pegboard::workflows::actor::StateUpdate),
	Upgrade,
	Destroy,
});
