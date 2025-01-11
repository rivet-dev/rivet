use std::{
	collections::{HashMap, HashSet},
	time::Duration,
};

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use pegboard::protocol as pp;
use serde_json::json;
use tokio::time::Instant;
use util::serde::AsHashableExt;

use super::{
	CreateComplete, Destroy, Drain, DrainState, Failed, GetServerMetaInput, GetServerMetaOutput,
	InsertDbInput, Port, Ready, SetConnectableInput, UpdateImageInput, UpdateRescheduleRetryInput,
	Upgrade, UpgradeComplete, UpgradeStarted, BASE_RETRY_TIMEOUT_MS, DRAIN_PADDING_MS,
};
use crate::types::{
	GameGuardProtocol, HostProtocol, NetworkMode, Routing, ServerLifecycle, ServerResources,
};

pub mod destroy;

#[derive(Serialize, Deserialize)]
struct StateRes {
	signal_actor: bool,
	override_kill_timeout_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
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
	match ctx.catch_unrecoverable(res)? {
		Ok(_actor_id) => {}
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
				override_kill_timeout_ms: None,
				signal_actor: false,
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

	let _client_id = match ctx.listen::<Init>().await? {
		Init::ActorStateUpdate(sig) => match sig.state {
			pp::ActorState::Allocated { client_id } => client_id,
			pp::ActorState::FailedToAllocate => {
				ctx.msg(Failed {
					message: "Failed to allocate (no availability).".into(),
				})
				.tag("server_id", input.server_id)
				.send()
				.await?;

				ctx.workflow(destroy::Input {
					server_id: input.server_id,
					override_kill_timeout_ms: None,
					signal_actor: false,
				})
				.output()
				.await?;

				bail!("failed to allocate actor");
			}
			state => bail!("unexpected actor state: {state:?}"),
		},
		Init::Destroy(sig) => {
			tracing::debug!("destroying before actor start");

			ctx.workflow(destroy::Input {
				server_id: input.server_id,
				override_kill_timeout_ms: sig.override_kill_timeout_ms,
				signal_actor: true,
			})
			.output()
			.await?;

			return Ok(());
		}
	};

	let state_res = ctx
		.repeat(|ctx| {
			let input = input.clone();

			async move {
				match ctx.listen::<Main>().await? {
					Main::ActorStateUpdate(sig) => match sig.state {
						pp::ActorState::Starting => {
							ctx.activity(SetStartedInput {
								server_id: input.server_id,
							})
							.await?;
						}
						pp::ActorState::Running { ports, .. } => {
							ctx.activity(UpdatePortsInput {
								server_id: input.server_id,
								datacenter_id: input.datacenter_id,
								ports,
							})
							.await?;

							// Wait for Traefik to poll ports and update GG
							let create_ts = ctx.ts();
							match ctx.check_version(2).await? {
								1 => ctx.removed::<Sleep>().await?,
								_latest => {
									ctx.activity(WaitForTraefikPollInput {
										create_ts,
										cluster_id: input.cluster_id,
										datacenter_id: input.datacenter_id,
									})
									.await?;
								}
							}

							let updated = ctx
								.activity(SetConnectableInput {
									server_id: input.server_id,
								})
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
							let exit_code = if let pp::ActorState::Exited { exit_code } = sig.state
							{
								exit_code
							} else {
								None
							};

							tracing::debug!(?exit_code, "actor stopped");

							let failed = exit_code.map(|exit_code| exit_code != 0).unwrap_or(true);

							// Reschedule durable actor if it errored
							if input.lifecycle.durable && failed {
								if let Some(sig) = reschedule_actor(ctx, &input, None).await? {
									// Destroyed early
									return Ok(Loop::Break(StateRes {
										signal_actor: true,
										override_kill_timeout_ms: sig.override_kill_timeout_ms,
									}));
								}
							} else {
								ctx.activity(SetFinishedInput {
									server_id: input.server_id,
								})
								.await?;

								return Ok(Loop::Break(StateRes {
									signal_actor: false,
									override_kill_timeout_ms: None,
								}));
							}
						}
						state => bail!("unexpected actor state: {state:?}"),
					},
					Main::Drain(sig) => {
						let drain_timeout = sig.drain_timeout.saturating_sub(DRAIN_PADDING_MS);
						let sleep_for = if drain_timeout < input.lifecycle.kill_timeout_ms {
							0
						} else {
							drain_timeout - input.lifecycle.kill_timeout_ms
						};

						match ctx.listen_with_timeout::<DrainState>(sleep_for).await? {
							Some(DrainState::Undrain(_)) => {}
							// Destroyed early
							Some(DrainState::Destroy(sig)) => {
								// TODO: Compare the override timeout to the remaining drain timeout and choose the
								// smaller one
								return Ok(Loop::Break(StateRes {
									signal_actor: true,
									override_kill_timeout_ms: sig.override_kill_timeout_ms,
								}));
							}
							// Drain timeout complete
							None => {
								// Reschedule durable actor on drain end
								if input.lifecycle.durable {
									// Important that we get the current actor id as durable actors can be
									// rescheduled many times
									let actor_id = ctx
										.activity(GetActorIdInput {
											server_id: input.server_id,
										})
										.await?;

									// Kill old actor immediately
									destroy::destroy_actor(
										ctx,
										input.datacenter_id,
										0,
										true,
										actor_id,
									)
									.await?;

									if let Some(sig) = reschedule_actor(ctx, &input, None).await? {
										// Destroyed early
										return Ok(Loop::Break(StateRes {
											signal_actor: true,
											override_kill_timeout_ms: sig.override_kill_timeout_ms,
										}));
									}
								} else {
									return Ok(Loop::Break(StateRes {
										signal_actor: true,
										override_kill_timeout_ms: Some(
											input.lifecycle.kill_timeout_ms.min(drain_timeout),
										),
									}));
								}
							}
						}
					}
					Main::Upgrade(sig) => {
						ctx.msg(UpgradeStarted {})
							.tag("server_id", input.server_id)
							.send()
							.await?;

						// Important that we get the current actor id as durable actors can be
						// rescheduled many times
						let actor_id = ctx
							.activity(GetActorIdInput {
								server_id: input.server_id,
							})
							.await?;

						// Kill old actor immediately
						destroy::destroy_actor(ctx, input.datacenter_id, 0, true, actor_id).await?;

						if let Some(sig) = reschedule_actor(ctx, &input, Some(sig.image_id)).await?
						{
							// Destroyed early
							return Ok(Loop::Break(StateRes {
								signal_actor: true,
								override_kill_timeout_ms: sig.override_kill_timeout_ms,
							}));
						}

						ctx.msg(UpgradeComplete {})
							.tag("server_id", input.server_id)
							.send()
							.await?;
					}
					Main::Destroy(sig) => {
						return Ok(Loop::Break(StateRes {
							signal_actor: true,
							override_kill_timeout_ms: sig.override_kill_timeout_ms,
						}))
					}
				}

				Ok(Loop::Continue)
			}
			.boxed()
		})
		.await?;

	ctx.workflow(destroy::Input {
		server_id: input.server_id,
		override_kill_timeout_ms: state_res.override_kill_timeout_ms,
		signal_actor: state_res.signal_actor,
	})
	.output()
	.await?;

	Ok(())
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
	let image_id = match &setup {
		SetupCtx::Init => {
			ctx.activity(InsertDbInput {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.as_hashable(),
				resources: input.resources.clone(),
				lifecycle: input.lifecycle.clone(),
				image_id: input.image_id,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.as_hashable(),
				network_ports: input.network_ports.as_hashable(),
			})
			.await?;

			input.image_id
		}
		SetupCtx::Reschedule { new_image_id } => {
			if let Some(image_id) = *new_image_id {
				ctx.activity(UpdateImageInput {
					server_id: input.server_id,
					image_id,
				})
				.await?;

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
			datacenter_id: input.datacenter_id,
		})
		.await?;

	let (actor_id, resources, artifacts_res) = ctx
		.join((
			activity(SelectActorIdInput {
				server_id: input.server_id,
			}),
			activity(SelectResourcesInput {
				datacenter_id: input.datacenter_id,
				resources: input.resources.clone(),
			}),
			activity(ResolveArtifactsInput {
				build_upload_id: server_meta.build_upload_id,
				build_file_name: server_meta.build_file_name.clone(),
				dc_build_delivery_method: server_meta.dc_build_delivery_method,
			}),
		))
		.await?;

	let actor_setup = ActorSetupCtx {
		actor_id,
		server_meta,
		resources,
		artifact_url_stub: artifacts_res.artifact_url_stub,
		fallback_artifact_url: artifacts_res.fallback_artifact_url,
	};

	// Rescheduling handles spawning the actor manually
	if let SetupCtx::Init = setup {
		spawn_actor(ctx, input, &actor_setup).await?;
	}

	Ok(actor_setup)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SelectActorIdInput {
	server_id: Uuid,
}

#[activity(SelectActorId)]
async fn select_actor_id(ctx: &ActivityCtx, input: &SelectActorIdInput) -> GlobalResult<Uuid> {
	let actor_id = Uuid::new_v4();

	sql_execute!(
		[ctx]
		"
		-- NOTE: We upsert here because the actor can be reassigned in the event of a reschedule
		UPSERT INTO db_ds.servers_pegboard (server_id, pegboard_actor_id)
		VALUES ($1, $2)
		",
		input.server_id,
		actor_id,
	)
	.await?;

	ctx.update_workflow_tags(&json!({
		"server_id": input.server_id,
		"actor_id": actor_id,
	}))
	.await?;

	Ok(actor_id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SelectResourcesInput {
	datacenter_id: Uuid,
	resources: ServerResources,
}

#[activity(SelectResources)]
async fn select_resources(
	ctx: &ActivityCtx,
	input: &SelectResourcesInput,
) -> GlobalResult<pp::Resources> {
	let tier_res = ctx
		.op(tier::ops::list::Input {
			datacenter_ids: vec![input.datacenter_id],
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

async fn spawn_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	actor_setup: &ActorSetupCtx,
) -> GlobalResult<()> {
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
				.map(|(port_label, port)| match port.routing {
					Routing::GameGuard { protocol, .. } => (
						crate::util::pegboard_normalize_port_label(port_label),
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
						crate::util::pegboard_normalize_port_label(port_label),
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
				cluster: pp::ActorMetadataCluster {
					cluster_id: input.cluster_id,
				},
				build: pp::ActorMetadataBuild {
					build_id: input.image_id,
				},
			})?,
		}),
	})
	.tag("datacenter_id", input.datacenter_id)
	.send()
	.await?;

	Ok(())
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
	let artifact_url_stub = format!(
		"/s3-cache/{namespace}-bucket-build/{upload_id}/{file_name}",
		namespace = ctx.config().server()?.rivet.namespace,
		upload_id = input.build_upload_id,
		file_name = input.build_file_name,
	);

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
		artifact_url_stub,
		fallback_artifact_url,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetStartedInput {
	server_id: Uuid,
}

#[activity(SetStarted)]
async fn set_started(ctx: &ActivityCtx, input: &SetStartedInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.servers
		SET start_ts = $2
		WHERE
			server_id = $1 AND
			start_ts IS NULL
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdatePortsInput {
	server_id: Uuid,
	datacenter_id: Uuid,
	ports: util::serde::HashableMap<String, pp::ProxiedPort>,
}

#[activity(UpdatePorts)]
async fn update_ports(ctx: &ActivityCtx, input: &UpdatePortsInput) -> GlobalResult<()> {
	let mut flat_port_labels = Vec::new();
	let mut flat_port_sources = Vec::new();
	let mut flat_port_ips = Vec::new();

	for (label, port) in &input.ports {
		flat_port_labels.push(label.as_str());
		flat_port_sources.push(port.source as i64);
		flat_port_ips.push(port.lan_hostname.clone());
	}

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_ds.server_proxied_ports (
			server_id,
			label,
			source,
			ip
		)
		SELECT $1, label, source, ip
		FROM unnest($2, $3, $4) AS n(label, source, ip)
		",
		input.server_id,
		flat_port_labels,
		flat_port_sources,
		flat_port_ips,
	)
	.await?;

	// Invalidate cache when ports are updated
	if !input.ports.is_empty() {
		ctx.cache()
			.purge("ds_proxied_ports2", [input.datacenter_id])
			.await?;
	}

	Ok(())
}

async fn reschedule_actor(
	ctx: &mut WorkflowCtx,
	input: &Input,
	new_image_id: Option<Uuid>,
) -> GlobalResult<Option<Destroy>> {
	tracing::info!("rescheduling actor");

	// Remove old proxied ports
	ctx.activity(ClearPortsInput {
		server_id: input.server_id,
	})
	.await?;

	let actor_setup = setup(ctx, &input, SetupCtx::Reschedule { new_image_id }).await?;

	// Waits for the actor to be ready (or destroyed) and automatically retries if failed to allocate.
	ctx.repeat(|ctx| {
		let input = input.clone();
		let actor_setup = actor_setup.clone();

		async move {
			// Get and increment retry count
			let retry_count = ctx
				.activity(UpdateRescheduleRetryInput {
					server_id: input.server_id,
				})
				.await?;

			// Don't sleep for first retry
			if retry_count > 0 {
				// Determine next backoff sleep duration
				let mut backoff = rivet_util::Backoff::new_at(
					8,
					None,
					BASE_RETRY_TIMEOUT_MS,
					500,
					(retry_count - 1).try_into()?,
				);
				let next = backoff.step().expect("should not have max retry");

				// Sleep for backoff or destroy early
				if let Some(sig) = ctx
					.listen_with_timeout::<Destroy>(next - Instant::now())
					.await?
				{
					tracing::debug!("destroying before actor start");

					return Ok(Loop::Break(Some(sig)));
				}
			}

			spawn_actor(ctx, &input, &actor_setup).await?;

			match ctx.listen::<Init>().await? {
				Init::ActorStateUpdate(sig) => match sig.state {
					pp::ActorState::Allocated {
						client_id: _client_id,
					} => return Ok(Loop::Break(None)),
					pp::ActorState::FailedToAllocate => return Ok(Loop::Continue),
					state => bail!("unexpected actor state: {state:?}"),
				},
				Init::Destroy(sig) => {
					tracing::debug!("destroying before actor start");

					return Ok(Loop::Break(Some(sig)));
				}
			};
		}
		.boxed()
	})
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearPortsInput {
	server_id: Uuid,
}

#[activity(ClearPorts)]
async fn clear_ports(ctx: &ActivityCtx, input: &ClearPortsInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		DELETE FROM db_ds.server_proxied_ports
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetFinishedInput {
	server_id: Uuid,
}

#[activity(SetFinished)]
async fn set_finished(ctx: &ActivityCtx, input: &SetFinishedInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_ds.servers
		SET finish_ts = $2
		WHERE server_id = $1
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetActorIdInput {
	server_id: Uuid,
}

#[activity(GetActorId)]
async fn get_actor_id(ctx: &ActivityCtx, input: &GetActorIdInput) -> GlobalResult<Uuid> {
	let (actor_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"
		SELECT pegboard_actor_id
		FROM db_ds.servers_pegboard
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	Ok(actor_id)
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
	cluster_id: Uuid,
	datacenter_id: Uuid,
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

	// Start sub first since the messages may arrive while fetching the server list
	let mut sub = ctx
		.subscribe::<TraefikPoll>(&json!({ "datacenter_id": input.datacenter_id }))
		.await?;

	// Fetch servers
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: cluster::types::Filter {
				pool_types: Some(vec![cluster::types::PoolType::Gg]),
				cluster_ids: Some(vec![input.cluster_id]),
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

join_signal!(Init {
	ActorStateUpdate(pegboard::workflows::client::ActorStateUpdate),
	Destroy,
});

join_signal!(Main {
	ActorStateUpdate(pegboard::workflows::client::ActorStateUpdate),
	Drain,
	Upgrade,
	Destroy,
});
