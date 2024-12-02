use std::collections::HashMap;

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use pegboard::protocol as pp;
use serde_json::json;
use util::serde::AsHashableExt;

use super::{
	resolve_image_artifact_url, CreateComplete, Destroy, Drain, DrainState, Failed,
	GetBuildAndDcInput, InsertDbInput, Port, Ready, SetConnectableInput, UpdateImageInput, Upgrade,
	UpgradeComplete, UpgradeStarted, DRAIN_PADDING_MS, TRAEFIK_GRACE_PERIOD,
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
	let res = setup(ctx, input, true, None).await;
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

	if let Some(sig) = wait_actor_ready(ctx, input.server_id).await? {
		// Destroyed early
		ctx.workflow(destroy::Input {
			server_id: input.server_id,
			override_kill_timeout_ms: sig.override_kill_timeout_ms,
			signal_actor: true,
		})
		.output()
		.await?;

		return Ok(());
	}

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

							// Wait for Traefik to be ready
							ctx.sleep(TRAEFIK_GRACE_PERIOD).await?;

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

async fn setup(
	ctx: &mut WorkflowCtx,
	input: &Input,
	insert_db: bool,
	new_image_id: Option<Uuid>,
) -> GlobalResult<Uuid> {
	if insert_db {
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
	} else if let Some(image_id) = new_image_id {
		ctx.activity(UpdateImageInput {
			server_id: input.server_id,
			image_id,
		})
		.await?;
	}

	let image_id = new_image_id.unwrap_or(input.image_id);

	let build_dc = ctx
		.activity(GetBuildAndDcInput {
			image_id,
			datacenter_id: input.datacenter_id,
		})
		.await?;

	let (actor_id, resources, image_artifact_url) = ctx
		.join((
			activity(SelectActorIdInput {
				server_id: input.server_id,
			}),
			activity(SelectResourcesInput {
				datacenter_id: input.datacenter_id,
				resources: input.resources.clone(),
			}),
			activity(ResolveArtifactsInput {
				datacenter_id: input.datacenter_id,
				image_id,
				server_id: input.server_id,
				build_upload_id: build_dc.build_upload_id,
				build_file_name: build_dc.build_file_name,
				dc_build_delivery_method: build_dc.dc_build_delivery_method,
			}),
		))
		.await?;

	ctx.signal(pp::Command::StartActor {
		actor_id,
		config: Box::new(pp::ActorConfig {
			image: pp::Image {
				artifact_url: image_artifact_url,
				kind: match build_dc.build_kind {
					BuildKind::DockerImage => pp::ImageKind::DockerImage,
					BuildKind::OciBundle => pp::ImageKind::OciBundle,
					BuildKind::JavaScript => pp::ImageKind::JavaScript,
				},
				compression: match build_dc.build_compression {
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
			resources,
			owner: pp::ActorOwner::DynamicServer {
				server_id: input.server_id,
			},
			metadata: util::serde::Raw::new(&json!({
				"actor": {
					"id": actor_id,
					"tags": input.tags,
					// Represents when the pegboard actor was created, not the ds workflow.
					"created_at": util::timestamp::to_string(ctx.ts())?,
				},
				"env": {
					"id": input.env_id,
				},
				"cluster": {
					"id": input.cluster_id
				},
				"region": {
					"name": build_dc.dc_name_id,
				},
				"build": {
					"id": input.image_id,
				},
			}))?,
		}),
	})
	.tag("datacenter_id", input.datacenter_id)
	.send()
	.await?;

	Ok(actor_id)
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

/// Returns the destroy signal if the dynamic server was destroyed.
async fn wait_actor_ready(ctx: &mut WorkflowCtx, server_id: Uuid) -> GlobalResult<Option<Destroy>> {
	let _client_id = match ctx.listen::<Init>().await? {
		Init::ActorStateUpdate(sig) => match sig.state {
			pp::ActorState::Allocated { client_id } => client_id,
			pp::ActorState::FailedToAllocate => {
				ctx.msg(Failed {
					message: "Failed to allocate (no availability).".into(),
				})
				.tag("server_id", server_id)
				.send()
				.await?;

				ctx.workflow(destroy::Input {
					server_id,
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

			return Ok(Some(sig));
		}
	};

	Ok(None)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ResolveArtifactsInput {
	datacenter_id: Uuid,
	image_id: Uuid,
	server_id: Uuid,
	build_upload_id: Uuid,
	build_file_name: String,
	dc_build_delivery_method: BuildDeliveryMethod,
}
#[activity(ResolveArtifacts)]
async fn resolve_artifacts(
	ctx: &ActivityCtx,
	input: &ResolveArtifactsInput,
) -> GlobalResult<String> {
	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![input.build_upload_id.into()],
	})
	.await?;
	let upload = unwrap!(upload_res.uploads.first());
	let upload_id = unwrap_ref!(upload.upload_id).as_uuid();

	let image_artifact_url = resolve_image_artifact_url(
		ctx,
		input.datacenter_id,
		input.build_file_name.clone(),
		input.dc_build_delivery_method,
		input.image_id,
		upload_id,
	)
	.await?;

	Ok(image_artifact_url)
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
		flat_port_ips.push(port.ip.to_string());
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
			.purge("ds_proxied_ports", [input.datacenter_id])
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

	let res = setup(ctx, &input, false, new_image_id).await;
	match ctx.catch_unrecoverable(res)? {
		Ok(_actor_id) => {}
		Err(err) => {
			tracing::error!(?err, "unrecoverable reschedule");

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

	// Wait for new actor to be ready
	wait_actor_ready(ctx, input.server_id).await
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
