use build::types::BuildKind;
use chirp_workflow::prelude::*;
use fdb_util::{FormalKey, SERIALIZABLE};
use foundationdb as fdb;
use nix::sys::signal::Signal;

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
		let client_workflow_id = actor.client_workflow_id;

		ctx.activity(UpdateFdbInput {
			actor_id: input.actor_id,
			build_kind: input.build_kind,
			actor,
		})
		.await?;

		if let (Some(client_workflow_id), Some(data)) = (client_workflow_id, &input.kill) {
			kill(
				ctx,
				input.actor_id,
				client_workflow_id,
				data.kill_timeout_ms,
				false,
			)
			.await?;
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

#[derive(Debug, Serialize, Deserialize, Hash, sqlx::FromRow)]
struct UpdateDbOutput {
	env_id: Uuid,
	selected_resources_memory_mib: Option<i64>,
	selected_resources_cpu_millicores: Option<i64>,
	tags: sqlx::types::Json<util::serde::HashableMap<String, String>>,
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
			selected_resources_memory_mib,
			selected_resources_cpu_millicores,
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
	build_kind: Option<BuildKind>,
	actor: UpdateDbOutput,
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
				let actor_key = keys::env::ActorKey::new(
					input.actor.env_id,
					input.actor.create_ts,
					input.actor_id,
				);
				let data = keys::env::ActorKeyData {
					is_destroyed: true,
					tags: input.actor.tags.0.clone().into_iter().collect(),
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

				if let Some(client_id) = input.actor.client_id {
					// This is cleared when the state changes as well as when the actor is destroyed to ensure
					// consistency during rescheduling and forced deletion.
					let actor_key = keys::client::ActorKey::new(client_id, input.actor_id);
					tx.clear(&keys::subspace().pack(&actor_key));
				}

				// Release client's resources and update allocation index
				if let (
					Some(build_kind),
					Some(client_id),
					Some(client_workflow_id),
					Some(selected_resources_memory_mib),
					Some(selected_resources_cpu_millicores),
				) = (
					input.build_kind,
					input.actor.client_id,
					input.actor.client_workflow_id,
					input.actor.selected_resources_memory_mib,
					input.actor.selected_resources_cpu_millicores,
				) {
					let client_flavor = match build_kind {
						BuildKind::DockerImage | BuildKind::OciBundle => {
							protocol::ClientFlavor::Container
						}
						BuildKind::JavaScript => protocol::ClientFlavor::Isolate,
					};

					let remaining_mem_key = keys::client::RemainingMemoryKey::new(client_id);
					let remaining_mem_key_buf = keys::subspace().pack(&remaining_mem_key);
					let remaining_cpu_key = keys::client::RemainingCpuKey::new(client_id);
					let remaining_cpu_key_buf = keys::subspace().pack(&remaining_cpu_key);
					let last_ping_ts_key = keys::client::LastPingTsKey::new(client_id);
					let last_ping_ts_key_buf = keys::subspace().pack(&last_ping_ts_key);

					let (remaining_mem_entry, remaining_cpu_entry, last_ping_ts_entry) = tokio::try_join!(
						tx.get(&remaining_mem_key_buf, SERIALIZABLE),
						tx.get(&remaining_cpu_key_buf, SERIALIZABLE),
						tx.get(&last_ping_ts_key_buf, SERIALIZABLE),
					)?;

					let remaining_mem = remaining_mem_key
						.deserialize(&remaining_mem_entry.ok_or(
							fdb::FdbBindingError::CustomError(
								format!("key should exist: {remaining_mem_key:?}").into(),
							),
						)?)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
					let remaining_cpu = remaining_cpu_key
						.deserialize(&remaining_cpu_entry.ok_or(
							fdb::FdbBindingError::CustomError(
								format!("key should exist: {remaining_cpu_key:?}").into(),
							),
						)?)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
					let last_ping_ts = last_ping_ts_key
						.deserialize(&last_ping_ts_entry.ok_or(
							fdb::FdbBindingError::CustomError(
								format!("key should exist: {last_ping_ts_key:?}").into(),
							),
						)?)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					let old_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
						client_flavor,
						remaining_mem,
						last_ping_ts,
						client_id,
					);
					let old_allocation_key_buf = keys::subspace().pack(&old_allocation_key);

					let new_mem = remaining_mem
						+ u64::try_from(selected_resources_memory_mib)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
					let new_cpu = remaining_cpu
						+ u64::try_from(selected_resources_cpu_millicores)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

					tracing::debug!(
						old_mem=%remaining_mem,
						old_cpu=%remaining_cpu,
						%new_mem,
						%new_cpu,
						"releasing resources"
					);

					// Write new memory
					tx.set(
						&remaining_mem_key_buf,
						&remaining_mem_key
							.serialize(new_mem)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);
					// Write new cpu
					tx.set(
						&remaining_cpu_key_buf,
						&remaining_cpu_key
							.serialize(new_cpu)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Only update allocation idx if it existed before
					if tx
						.get(&old_allocation_key_buf, SERIALIZABLE)
						.await?
						.is_some()
					{
						// Clear old key
						tx.clear(&old_allocation_key_buf);

						let new_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
							client_flavor,
							new_mem,
							last_ping_ts,
							client_id,
						);
						let new_allocation_key_buf = keys::subspace().pack(&new_allocation_key);

						tx.set(
							&new_allocation_key_buf,
							&new_allocation_key
								.serialize(client_workflow_id)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}
				}

				Ok(())
			}
		})
		.await?;

	Ok(())
}

pub(crate) async fn kill(
	ctx: &mut WorkflowCtx,
	actor_id: Uuid,
	client_workflow_id: Uuid,
	kill_timeout_ms: i64,
	persist_storage: bool,
) -> GlobalResult<()> {
	if kill_timeout_ms != 0 {
		ctx.signal(protocol::Command::SignalActor {
			actor_id,
			signal: Signal::SIGTERM as i32,
			persist_storage,
		})
		.to_workflow_id(client_workflow_id)
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
	.to_workflow_id(client_workflow_id)
	.send()
	.await?;

	Ok(())
}
