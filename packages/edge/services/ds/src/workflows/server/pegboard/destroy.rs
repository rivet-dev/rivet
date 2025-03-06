use std::collections::HashMap;

use chirp_workflow::prelude::*;
use fdb_util::FormalKey;
use foundationdb as fdb;
use nix::sys::signal::Signal;
use pegboard::protocol as pp;
use util::serde::AsHashableExt;

use super::super::{DestroyComplete, DestroyStarted};
use crate::{keys, types::GameGuardProtocol};

#[derive(Debug, Serialize, Deserialize)]
pub struct DestroyActor {
	pub actor_id: Uuid,
	pub client_id: Uuid,
	pub kill_timeout_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	/// Whether or not to send signals to the pb actor. In the case that the actor was already stopped
	/// or exited, signals are unnecessary.
	pub destroy_actor: Option<DestroyActor>,
}

#[workflow]
pub(crate) async fn ds_server_pegboard_destroy(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	let ds = ctx.activity(UpdateDbInput {}).await?;

	if let Some(ds) = ds {
		ctx.activity(UpdateFdbInput {
			server_id: input.server_id,
			env_id: ds.env_id,
			tags: ds.tags.as_hashable(),
			create_ts: ds.create_ts,
		})
		.await?;
	}

	ctx.msg(DestroyStarted {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	if let Some(data) = &input.destroy_actor {
		destroy_actor(
			ctx,
			data.actor_id,
			data.client_id,
			data.kill_timeout_ms,
			false,
		)
		.await?;
	}

	ctx.msg(DestroyComplete {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct UpdateDbOutput {
	env_id: Uuid,
	tags: sqlx::types::Json<HashMap<String, String>>,
	create_ts: i64,
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
		RETURNING env_id, json(tags) AS tags, create_ts
		",
		ctx.ts(),
	)
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateFdbInput {
	server_id: Uuid,
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
		FROM server_ports_ingress
		",
	)
	.await?;

	ctx.fdb()
		.await?
		.run(|tx, _mc| {
			let ingress_ports = ingress_ports.clone();
			async move {
				// Update server key in env subspace
				let server_key =
					keys::env::ServerKey::new(input.env_id, input.create_ts, input.server_id);
				let data = keys::env::ServerKeyData {
					is_destroyed: true,
					tags: input.tags.clone().into_iter().collect(),
				};
				tx.set(
					&keys::subspace().pack(&server_key),
					&server_key
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
						input.server_id,
					);

					tx.clear(&keys::subspace().pack(&ingress_port_key));
				}

				// Remove proxied ports
				let proxied_ports_key = keys::server::ProxiedPortsKey::new(input.server_id);
				tx.clear(&keys::subspace().pack(&proxied_ports_key));

				Ok(())
			}
		})
		.await?;

	Ok(())
}

pub(crate) async fn destroy_actor(
	ctx: &mut WorkflowCtx,
	actor_id: Uuid,
	client_id: Uuid,
	kill_timeout_ms: i64,
	persist_storage: bool,
) -> GlobalResult<()> {
	if kill_timeout_ms != 0 {
		ctx.signal(pp::Command::SignalActor {
			actor_id,
			signal: Signal::SIGTERM as i32,
			persist_storage,
			ignore_future_state: true,
		})
		.tag("client_id", client_id)
		.send()
		.await?;

		// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
		ctx.sleep(kill_timeout_ms).await?;
	}

	ctx.signal(pp::Command::SignalActor {
		actor_id,
		signal: Signal::SIGKILL as i32,
		persist_storage,
		ignore_future_state: true,
	})
	.tag("client_id", client_id)
	.send()
	.await?;

	Ok(())
}
