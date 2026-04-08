use rivet_envoy_protocol as protocol;

use crate::connection::ws_send;
use crate::envoy::EnvoyContext;

pub async fn handle_send_events(ctx: &mut EnvoyContext, events: Vec<protocol::EventWrapper>) {
	// Record in history per actor
	for event in &events {
		let entry =
			ctx.get_actor_entry_mut(&event.checkpoint.actor_id, event.checkpoint.generation);
		if let Some(entry) = entry {
			entry.event_history.push(event.clone());

			// Close the actor channel but keep event history for ack/resend.
			if let protocol::Event::EventActorStateUpdate(ref state_update) = event.inner {
				if matches!(
					state_update.state,
					protocol::ActorState::ActorStateStopped(_)
				) {
					// Mark handle as done - actor task will finish on its own
				}
			}
		}
	}

	// Send if connected
	ws_send(&ctx.shared, protocol::ToRivet::ToRivetEvents(events)).await;
}

pub fn handle_ack_events(ctx: &mut EnvoyContext, ack: protocol::ToEnvoyAckEvents) {
	for checkpoint in &ack.last_event_checkpoints {
		let entry = ctx.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation);
		if let Some(entry) = entry {
			entry
				.event_history
				.retain(|event| event.checkpoint.index > checkpoint.index);

			// Clean up fully acked stopped actors
			if entry.event_history.is_empty() && entry.handle.is_closed() {
				// Need to remove after loop
			}
		}
	}

	// Clean up fully acked stopped actors
	for checkpoint in &ack.last_event_checkpoints {
		let should_remove_gen = ctx
			.actors
			.get(&checkpoint.actor_id)
			.and_then(|gens| gens.get(&checkpoint.generation))
			.map(|entry| entry.event_history.is_empty() && entry.handle.is_closed())
			.unwrap_or(false);

		if should_remove_gen {
			if let Some(gens) = ctx.actors.get_mut(&checkpoint.actor_id) {
				gens.remove(&checkpoint.generation);
				if gens.is_empty() {
					ctx.actors.remove(&checkpoint.actor_id);
				}
			}
		}
	}
}

pub async fn resend_unacknowledged_events(ctx: &EnvoyContext) {
	let mut events: Vec<protocol::EventWrapper> = Vec::new();

	for generations in ctx.actors.values() {
		for entry in generations.values() {
			events.extend(entry.event_history.iter().cloned());
		}
	}

	if events.is_empty() {
		return;
	}

	tracing::info!(count = events.len(), "resending unacknowledged events");

	ws_send(&ctx.shared, protocol::ToRivet::ToRivetEvents(events)).await;
}
