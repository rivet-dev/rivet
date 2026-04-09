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

			if let protocol::Event::EventActorStateUpdate(ref state_update) = event.inner {
				if matches!(
					state_update.state,
					protocol::ActorState::ActorStateStopped(_)
				) {
					// If the actor is being stopped by rivet, we don't need the entry anymore
					if entry.received_stop {
						ctx.actors.remove(&event.checkpoint.actor_id);
					}
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
		}
	}
}

// TODO: If the envoy disconnects, actor stops, then envoy reconnects, we will send the stop event but there
// is no mechanism to remove the actor entry afterwards. We only remove the actor entry if rivet stops the actor.
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
