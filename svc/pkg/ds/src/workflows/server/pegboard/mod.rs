use std::collections::HashMap;

use build::types::{BuildCompression, BuildKind};
use chirp_workflow::prelude::*;
use cluster::types::BuildDeliveryMethod;
use futures_util::FutureExt;
use pegboard::protocol as pp;
use serde_json::json;
use util::serde::AsHashableExt;

use super::{
	resolve_image_artifact_url, CreateComplete, CreateFailed, Destroy, Drain, DrainState,
	GetBuildAndDcInput, InsertDbInput, Port, DRAIN_PADDING_MS, TRAEFIK_GRACE_PERIOD,
};
use crate::types::{GameGuardProtocol, NetworkMode, Routing, ServerResources};

pub mod destroy;

#[derive(Serialize, Deserialize)]
struct StateRes {
	signal: bool,
	override_kill_timeout_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub kill_timeout_ms: i64,
	pub image_id: Uuid,
	pub root_user_enabled: bool,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[workflow]
pub(crate) async fn ds_server_pegboard(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let res = setup(ctx, input).await;
	match ctx.catch_unrecoverable(res)? {
		Ok(x) => x,
		// If we cannot recover a setup error, send a failed signal
		Err(err) => {
			tracing::warn!(?err, "unrecoverable setup");

			// TODO: Cleanup

			ctx.msg(CreateFailed {})
				.tag("server_id", input.server_id)
				.send()
				.await?;

			// Throw the original error from the setup activities
			return Err(err);
		}
	};

	ctx.msg(CreateComplete {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	// Wait for actor start
	match ctx.listen::<Init>().await? {
		Init::ActorStateUpdate(sig) => match sig.state {
			pp::ActorState::Allocated { client_id } => client_id,
			pp::ActorState::FailedToAllocate => {
				// TODO: Return error from wf
				bail!("failed to allocate actor");
			}
			state => bail!("unexpected actor state: {state:?}"),
		},
		Init::Destroy(sig) => {
			tracing::info!("destroying before actor start");

			ctx.workflow(destroy::Input {
				server_id: input.server_id,
				override_kill_timeout_ms: sig.override_kill_timeout_ms,
				signal: true,
			})
			.output()
			.await?;

			return Ok(());
		}
	};

	let state_res = ctx
		.repeat(|ctx| {
			let server_id = input.server_id;
			let datacenter_id = input.datacenter_id;
			let kill_timeout_ms = input.kill_timeout_ms;

			async move {
				match ctx.listen::<Main>().await? {
					Main::ActorStateUpdate(sig) => match sig.state {
						pp::ActorState::Starting => {
							ctx.activity(SetStartedInput { server_id }).await?;
						}
						pp::ActorState::Running { ports, .. } => {
							// Wait for Traefik to be ready
							ctx.sleep(TRAEFIK_GRACE_PERIOD).await?;

							ctx.activity(UpdatePortsInput {
								server_id,
								datacenter_id,
								ports,
							})
							.await?;
						}
						pp::ActorState::Stopping
						| pp::ActorState::Stopped
						| pp::ActorState::Exited { .. } => {
							tracing::info!("actor stopped");

							ctx.activity(SetFinishedInput { server_id }).await?;

							return Ok(Loop::Break(StateRes {
								signal: false,
								override_kill_timeout_ms: None,
							}));
						}
						state => bail!("unexpected actor state: {state:?}"),
					},
					Main::Drain(sig) => {
						let drain_timeout = sig.drain_timeout.saturating_sub(DRAIN_PADDING_MS);
						let sleep_for = if drain_timeout < kill_timeout_ms {
							0
						} else {
							drain_timeout - kill_timeout_ms
						};

						match ctx.listen_with_timeout::<DrainState>(sleep_for).await? {
							Some(DrainState::Undrain(_)) => {}
							// TODO: Compare the override timeout to the remaining drain timeout and choose the
							// smaller one
							Some(DrainState::Destroy(sig)) => {
								return Ok(Loop::Break(StateRes {
									signal: true,
									override_kill_timeout_ms: sig.override_kill_timeout_ms,
								}));
							}
							// Drain timeout complete
							None => {
								return Ok(Loop::Break(StateRes {
									signal: true,
									override_kill_timeout_ms: Some(
										kill_timeout_ms.min(drain_timeout),
									),
								}));
							}
						}
					}
					Main::Destroy(sig) => {
						return Ok(Loop::Break(StateRes {
							signal: true,
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
		signal: state_res.signal,
	})
	.output()
	.await?;

	Ok(())
}

async fn setup(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<Uuid> {
	let (_, build_dc) = ctx
		.join((
			activity(InsertDbInput {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.as_hashable(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
				image_id: input.image_id,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.as_hashable(),
				network_ports: input.network_ports.as_hashable(),
			}),
			activity(GetBuildAndDcInput {
				image_id: input.image_id,
				datacenter_id: input.datacenter_id,
			}),
		))
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
				image_id: input.image_id,
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
					Routing::GameGuard { protocol } => {
						// Must be present for GG routing
						let target = unwrap!(port.internal_port) as u16;

						Ok((
							crate::util::format_port_label(port_label),
							pp::Port::GameGuard {
								target,
								protocol: match protocol {
									GameGuardProtocol::Http
									| GameGuardProtocol::Https
									| GameGuardProtocol::Tcp
									| GameGuardProtocol::TcpTls => pp::TransportProtocol::Tcp,
									GameGuardProtocol::Udp => pp::TransportProtocol::Udp,
								},
							},
						))
					}
					Routing::Host { .. } => {
						// TODO:
						bail!("host ports not implemented");
					}
				})
				.collect::<GlobalResult<_>>()?,
			network_mode: match input.network_mode {
				NetworkMode::Bridge => pp::NetworkMode::Bridge,
				NetworkMode::Host => pp::NetworkMode::Host,
			},
			resources,
			stakeholder: pp::Stakeholder::DynamicServer {
				server_id: input.server_id,
			},
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
		INSERT INTO db_ds.servers_pegboard (server_id, pegboard_actor_id)
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
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdatePortsInput {
	server_id: Uuid,
	datacenter_id: Uuid,
	ports: util::serde::HashableMap<String, pp::BoundPort>,
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
		WITH
			update_server AS (
				UPDATE db_ds.servers
				SET connectable_ts = $2
				WHERE
					server_id = $1 AND
					connectable_ts IS NULL
				RETURNING 1
			),
			insert_ports AS (
				INSERT INTO db_ds.internal_ports (
					server_id,
					label,
					source,
					ip
				)
				SELECT $1, label, source, ip
				FROM unnest($3, $4, $5) AS n(label, source, ip)
				WHERE EXISTS(
					SELECT 1 FROM update_server
				)
				RETURNING 1
			)
		SELECT 1
		",
		input.server_id,
		util::timestamp::now(),
		flat_port_labels,
		flat_port_sources,
		flat_port_ips,
	)
	.await?;

	// Invalidate cache when ports are updated
	if !input.ports.is_empty() {
		ctx.cache()
			.purge("servers_ports", [input.datacenter_id])
			.await?;
	}

	Ok(())
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
		WHERE server_id = $1
		",
		input.server_id,
		util::timestamp::now(),
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

join_signal!(Init {
	ActorStateUpdate(pegboard::workflows::client::ActorStateUpdate),
	Destroy,
});

join_signal!(Main {
	ActorStateUpdate(pegboard::workflows::client::ActorStateUpdate),
	Destroy,
	Drain,
});
