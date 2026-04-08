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
		}
	}

	// Send if connected
	ws_send(&ctx.shared, protocol::ToRivet::ToRivetEvents(events)).await;
}

pub fn handle_ack_events(ctx: &mut EnvoyContext, ack: protocol::ToEnvoyAckEvents) {
	for checkpoint in &ack.last_event_checkpoints {
		let actor_entry = ctx.actors.get_mut(&checkpoint.actor_id);
		if let Some(actor_entry) = actor_entry {
			let gen_entry = actor_entry.get_mut(&checkpoint.generation);
			let remove = if let Some(gen_entry) = gen_entry {
				gen_entry
					.event_history
					.retain(|event| event.checkpoint.index > checkpoint.index);

				gen_entry.event_history.is_empty() && gen_entry.handle.is_closed()
			} else {
				false
			};

			// Clean up fully acked stopped actors
			if remove {
				actor_entry.remove(&checkpoint.generation);
				if actor_entry.is_empty() {
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
