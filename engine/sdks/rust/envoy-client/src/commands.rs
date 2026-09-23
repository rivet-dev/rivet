use std::collections::HashMap;

use rivet_envoy_protocol as protocol;

use crate::actor::create_actor_with_startup;
use crate::connection::ws_send;
use crate::envoy::EnvoyContext;
use crate::stringify::stringify_command_wrapper;

pub const ACK_COMMANDS_INTERVAL_MS: u64 = 5 * 60 * 1000;

pub async fn handle_commands(ctx: &mut EnvoyContext, commands: Vec<protocol::CommandWrapper>) {
	tracing::info!(command_count = commands.len(), "received commands");
	for command_wrapper in &commands {
		tracing::info!(
			command = %stringify_command_wrapper(command_wrapper),
			"received command"
		);
	}

	// Collect every actor in the raw batch before dedup, so a replayed
	// (skipped) command is still re-acked instead of being replayed forever.
	let batch_actors: Vec<(String, u32)> = commands
		.iter()
		.map(|c| (c.checkpoint.actor_id.clone(), c.checkpoint.generation))
		.collect();

	for mut command_wrapper in commands {
		let waiting = match &mut command_wrapper.inner {
			protocol::Command::CommandStartActor(start) => {
				std::mem::take(&mut start.waiting_requests)
			}
			_ => Vec::new(),
		};
		let checkpoint = command_wrapper.checkpoint;
		let dedup_key = (checkpoint.actor_id.clone(), checkpoint.generation);

		let previous_generation = ctx
			.processed_command_idx
			.keys()
			.filter(|(id, _)| id == &checkpoint.actor_id)
			.map(|(_, g)| *g)
			.max();
		let replay = previous_generation.is_some_and(|g| g > checkpoint.generation)
			|| ctx
				.processed_command_idx
				.get(&dedup_key)
				.is_some_and(|index| checkpoint.index <= *index);
		if !replay {
			ctx.processed_command_idx
				.retain(|(id, g), _| id != &checkpoint.actor_id || *g >= checkpoint.generation);
			ctx.processed_command_idx
				.insert(dedup_key, checkpoint.index);

			match command_wrapper.inner {
				protocol::Command::CommandStartActor(val) => {
					if ctx
						.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation)
						.is_none()
					{
						let actor_name = val.config.name.clone();
						let (handle, active_http_request_count) = create_actor_with_startup(
							ctx.shared.clone(),
							checkpoint.actor_id.clone(),
							checkpoint.generation,
							val.config,
							val.hibernating_requests,
							val.preloaded_kv,
							val.sqlite_startup,
						);

						ctx.insert_actor(
							checkpoint.actor_id.clone(),
							checkpoint.generation,
							handle,
							active_http_request_count,
							actor_name,
							checkpoint.index,
						);
					}
				}
				protocol::Command::CommandStopActor(val) => {
					let entry =
						ctx.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation);

					if let Some(entry) = entry {
						entry.received_stop = true;
						entry.last_command_idx = checkpoint.index;
						let _ = entry.handle.send(crate::actor::ToActor::Stop {
							command_idx: checkpoint.index,
							reason: val.reason,
						});
					} else {
						tracing::warn!(
							actor_id = %checkpoint.actor_id,
							generation = checkpoint.generation,
							"received stop actor command for unknown actor"
						);
					}
				}
			}
		}
		for request in waiting {
			admit_startup_request(ctx, &checkpoint, request).await;
		}
	}

	// Ack the whole batch immediately. Anything left unacked stays in the
	// engine's `ActorCommandKey` subspace, and `envoy_conn_prepare` re-streams
	// that subspace on every reconnect, so a start that waits for the periodic
	// tick can be replayed for up to `ACK_COMMANDS_INTERVAL_MS` and resurrect a
	// stopped actor or replace a live one. Scope to just this batch's actors
	// instead of a full-state ack, and do not clear dedup; the tick handles
	// full re-acks, recovery, and clearing.
	if !batch_actors.is_empty() {
		send_batch_command_acks(ctx, &batch_actors).await;
	}
}

/// Ack only the given actors' latest processed command index. Used for the
/// immediate post-batch ack. Does not clear dedup (see the race note in
/// `send_command_ack`); a failed send is retried by the replayed batch or tick.
async fn send_batch_command_acks(ctx: &EnvoyContext, actors: &[(String, u32)]) {
	let mut highest: HashMap<(String, u32), i64> = HashMap::new();
	for key in actors {
		if let Some(&index) = ctx.processed_command_idx.get(key) {
			highest.insert(key.clone(), index);
		}
	}

	if highest.is_empty() {
		return;
	}

	send_ack_checkpoints(ctx, checkpoints_from(highest)).await;
}

pub async fn send_command_ack(ctx: &mut EnvoyContext) {
	// Merge live actors and the dedup map, highest index per actor-generation.
	// Live actors are re-acked every tick (recovers an ack accepted locally but
	// never committed by the server); the dedup map covers stops whose actor was
	// already removed and is cleared once acked.
	let mut highest: HashMap<(String, u32), i64> = HashMap::new();
	for (actor_id, generations) in &ctx.actors {
		for (generation, entry) in generations {
			if entry.last_command_idx >= 0 {
				highest.insert((actor_id.clone(), *generation), entry.last_command_idx);
			}
		}
	}
	for ((actor_id, generation), &index) in &ctx.processed_command_idx {
		highest
			.entry((actor_id.clone(), *generation))
			.and_modify(|existing| *existing = (*existing).max(index))
			.or_insert(index);
	}

	if highest.is_empty() {
		return;
	}

	let last_command_checkpoints = checkpoints_from(highest);
	let send_failed = send_ack_checkpoints(ctx, last_command_checkpoints.clone()).await;

	// Skip the dedup clear if the ack never left this process. Otherwise
	// `pegboard-envoy` would replay the commands on reconnect with no dedup
	// state to suppress them.
	if send_failed {
		return;
	}

	// Keep one generation watermark per actor for this process lifetime. Sending an ACK is
	// not proof that the engine durably deleted a command; delayed Start cannot resurrect it.
}

fn checkpoints_from(highest: HashMap<(String, u32), i64>) -> Vec<protocol::ActorCheckpoint> {
	highest
		.into_iter()
		.map(
			|((actor_id, generation), index)| protocol::ActorCheckpoint {
				actor_id,
				generation,
				index,
			},
		)
		.collect()
}

/// Send an ack for the given checkpoints. Returns whether the send failed.
async fn send_ack_checkpoints(
	ctx: &EnvoyContext,
	last_command_checkpoints: Vec<protocol::ActorCheckpoint>,
) -> bool {
	ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetAckCommands(protocol::ToRivetAckCommands {
			last_command_checkpoints,
		}),
	)
	.await
}

async fn admit_startup_request(
	ctx: &mut EnvoyContext,
	checkpoint: &protocol::ActorCheckpoint,
	request: protocol::ToEnvoyTunnelMessage,
) {
	let session = ctx
		.shared
		.connection_session
		.load(std::sync::atomic::Ordering::Acquire);
	let valid = matches!(&request.message_kind, protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(r)
        if r.actor_id == checkpoint.actor_id && r.actor_generation == Some(checkpoint.generation));
	if !valid {
		return;
	}
	let mut key = [0; 8];
	key[..4].copy_from_slice(&request.message_id.gateway_id);
	key[4..].copy_from_slice(&request.message_id.request_id);
	if let Some(entry) = ctx.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation) {
		if entry.startup_requests.contains(&key) {
			return;
		}
		if entry.startup_requests.len() >= 128 {
			crate::tunnel::send_response_abort_for_session(
				ctx,
				session,
				request.message_id,
				"actor startup request capacity exceeded",
			)
			.await;
			return;
		}
		entry.startup_requests.insert(key);
	}
	crate::tunnel::handle_tunnel_message(ctx, session, request).await;
}
