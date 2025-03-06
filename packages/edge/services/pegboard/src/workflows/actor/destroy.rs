use std::collections::HashMap;

use build::types::BuildKind;
use chirp_workflow::prelude::*;
use fdb_util::FormalKey;
use foundationdb as fdb;
use nix::sys::signal::Signal;
use util::serde::AsHashableExt;

use super::{DestroyComplete, DestroyStarted};
use crate::{keys, protocol, types::GameGuardProtocol};

#[derive(Debug, Serialize, Deserialize)]
pub struct KillCtx {
	pub kill_timeout_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub actor_id: Uuid,
	pub build_kind: Option<BuildKind>,
	/// Whether or not to send signals to the pb actor. In the case that the actor was already stopped
	/// or exited, signals are unnecessary.
	pub kill: Option<KillCtx>,
}

#[workflow]
pub(crate) async fn pegboard_actor_destroy(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	ctx.msg(DestroyStarted {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	let actor = ctx.activity(UpdateDbInput {}).await?;

	if let Some(actor) = actor {
		ctx.join((
			activity(UpdateFdbInput {
				actor_id: input.actor_id,
				env_id: actor.env_id,
				tags: actor.tags.as_hashable(),
				create_ts: actor.create_ts,
			}),
			if let (Some(build_kind), Some(client_id), Some(client_workflow_id)) =
				(input.build_kind, actor.client_id, actor.client_workflow_id)
			{
				Some(activity(ReleaseResourcesInput {
					client_id,
					client_workflow_id,
					build_kind,
					memory: actor.resources_memory_mib.try_into()?,
					cpu: actor.resources_cpu_millicores.try_into()?,
				}))
			} else {
				None
			},
		))
		.await?;

		if let (Some(client_id), Some(data)) = (actor.client_id, &input.kill) {
			kill(ctx, input.actor_id, client_id, data.kill_timeout_ms, false).await?;
		}
	}

	ctx.msg(DestroyComplete {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct UpdateDbOutput {
	env_id: Uuid,
	resources_memory_mib: i64,
	resources_cpu_millicores: i64,
	tags: sqlx::types::Json<HashMap<String, String>>,
	create_ts: i64,
	client_id: Option<Uuid>,
	client_workflow_id: Option<Uuid>,
}

#[activity(UpdateDb)]
async fn update_db(
	ctx: &ActivityCtx,
	input: &UpdateDbInput,
) -> GlobalResult<Option<UpdateDbOutput>> {
	let pool = ctx.sqlite().await?;

	sql_fetch_optional!(
		[ctx, UpdateDbOutput, pool]
		"
		UPDATE state
		SET destroy_ts = ?
		WHERE destroy_ts IS NULL
		RETURNING
			env_id,
			resources_memory_mib,
			resources_cpu_millicores,
			json(tags) AS tags,
			create_ts,
			client_id,
			client_workflow_id
		",
		ctx.ts(),
	)
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateFdbInput {
	actor_id: Uuid,
	env_id: Uuid,
	tags: util::serde::HashableMap<String, String>,
	create_ts: i64,
}

#[activity(UpdateFdb)]
async fn update_fdb(ctx: &ActivityCtx, input: &UpdateFdbInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;

	let ingress_ports = sql_fetch_all!(
		[ctx, (i64, i64), pool]
		"
		SELECT protocol, ingress_port_number
		FROM ports_ingress
		",
	)
	.await?;

	ctx.fdb()
		.await?
		.run(|tx, _mc| {
			let ingress_ports = ingress_ports.clone();
			async move {
				// Update actor key in env subspace
				let actor_key =
					keys::env::ActorKey::new(input.env_id, input.create_ts, input.actor_id);
				let data = keys::env::ActorKeyData {
					is_destroyed: true,
					tags: input.tags.clone().into_iter().collect(),
				};
				tx.set(
					&keys::subspace().pack(&actor_key),
					&actor_key
						.serialize(data)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				// Remove all allocated ingress ports
				for (protocol, port) in ingress_ports {
					let ingress_port_key = keys::port::IngressKey::new(
						GameGuardProtocol::from_repr(
							usize::try_from(protocol)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						)
						.ok_or_else(|| {
							fdb::FdbBindingError::CustomError(
								format!("invalid protocol variant: {protocol}").into(),
							)
						})?,
						u16::try_from(port)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						input.actor_id,
					);

					tx.clear(&keys::subspace().pack(&ingress_port_key));
				}

				// Remove proxied ports
				let proxied_ports_key = keys::actor::ProxiedPortsKey::new(input.actor_id);
				tx.clear(&keys::subspace().pack(&proxied_ports_key));

				Ok(())
			}
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ReleaseResourcesInput {
	client_id: Uuid,
	client_workflow_id: Uuid,
	build_kind: BuildKind,
	/// MiB.
	memory: u64,
	/// Millicores.
	cpu: u64,
}

#[activity(ReleaseResources)]
async fn release_resources(ctx: &ActivityCtx, input: &ReleaseResourcesInput) -> GlobalResult<()> {
	let client_flavor = match input.build_kind {
		BuildKind::DockerImage | BuildKind::OciBundle => protocol::ClientFlavor::Container,
		BuildKind::JavaScript => protocol::ClientFlavor::Isolate,
	};

	ctx.op(crate::ops::client::update_allocation_idx::Input {
		client_id: input.client_id,
		client_workflow_id: input.client_workflow_id,
		flavor: client_flavor,
		action: crate::ops::client::update_allocation_idx::Action::ReleaseResources {
			memory: input.memory,
			cpu: input.cpu,
		},
	})
	.await
}

pub(crate) async fn kill(
	ctx: &mut WorkflowCtx,
	actor_id: Uuid,
	client_id: Uuid,
	kill_timeout_ms: i64,
	persist_storage: bool,
) -> GlobalResult<()> {
	if kill_timeout_ms != 0 {
		ctx.signal(protocol::Command::SignalActor {
			actor_id,
			signal: Signal::SIGTERM as i32,
			persist_storage,
		})
		.tag("client_id", client_id)
		.send()
		.await?;

		// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
		ctx.sleep(kill_timeout_ms).await?;
	}

	ctx.signal(protocol::Command::SignalActor {
		actor_id,
		signal: Signal::SIGKILL as i32,
		persist_storage,
	})
	.tag("client_id", client_id)
	.send()
	.await?;

	Ok(())
}
