use rivet_envoy_protocol as protocol;

use crate::actor::create_actor;
use crate::connection::ws_send;
use crate::envoy::{ActorEntry, EnvoyContext};

pub const ACK_COMMANDS_INTERVAL_MS: u64 = 5 * 60 * 1000;

pub async fn handle_commands(ctx: &mut EnvoyContext, commands: Vec<protocol::CommandWrapper>) {
	tracing::info!(command_count = commands.len(), "received commands");

	for command_wrapper in commands {
		let checkpoint = command_wrapper.checkpoint;
		match command_wrapper.inner {
			protocol::Command::CommandStartActor(val) => {
				let actor_name = val.config.name.clone();
				let handle = create_actor(
					ctx.shared.clone(),
					checkpoint.actor_id.clone(),
					checkpoint.generation,
					val.config,
					val.hibernating_requests,
					val.preloaded_kv,
				);

				let generations = ctx
					.actors
					.entry(checkpoint.actor_id.clone())
					.or_insert_with(std::collections::HashMap::new);
				generations.insert(
					checkpoint.generation,
					ActorEntry {
						handle,
						name: actor_name,
						event_history: Vec::new(),
						last_command_idx: checkpoint.index,
						received_stop: false,
					},
				);
			}
			protocol::Command::CommandStopActor(val) => {
				let entry = ctx.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation);

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
}

pub async fn send_command_ack(ctx: &EnvoyContext) {
	let mut last_command_checkpoints: Vec<protocol::ActorCheckpoint> = Vec::new();

	for (actor_id, generations) in &ctx.actors {
		for (generation, entry) in generations {
			if entry.last_command_idx < 0 {
				continue;
			}
			last_command_checkpoints.push(protocol::ActorCheckpoint {
				actor_id: actor_id.clone(),
				generation: *generation,
				index: entry.last_command_idx,
			});
		}
	}

	if last_command_checkpoints.is_empty() {
		return;
	}

	ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetAckCommands(protocol::ToRivetAckCommands {
			last_command_checkpoints,
		}),
	)
	.await;
}
