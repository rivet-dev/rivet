use rivet_envoy_protocol as protocol;

use crate::connection::ws_send;
use crate::deadlines::DeadlineKey;
use crate::envoy::EnvoyContext;
use crate::stringify::stringify_event_wrapper;

pub(crate) const TERMINAL_EVENT_RETENTION: std::time::Duration =
	std::time::Duration::from_secs(60 * 60);

pub async fn handle_send_events(ctx: &mut EnvoyContext, events: Vec<protocol::EventWrapper>) {
	tracing::info!(event_count = events.len(), "sending events");
	for event in &events {
		tracing::info!(event = %stringify_event_wrapper(event), "sending event");
	}

	for event in &events {
		let pending = ctx
			.pending_events
			.entry((
				event.checkpoint.actor_id.clone().to_string(),
				event.checkpoint.generation,
			))
			.or_default();
		pending.events.push(event.clone());
		if matches!(
			&event.inner,
			protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
				state: protocol::ActorState::ActorStateStopped(_),
			})
		) {
			retire(ctx, &event.checkpoint.actor_id, event.checkpoint.generation);
		}
		let mut remove_after_stop = false;
		let entry =
			ctx.get_actor_entry_mut(&event.checkpoint.actor_id, event.checkpoint.generation);
		if let Some(entry) = entry {
			if let protocol::Event::EventActorStateUpdate(ref state_update) = event.inner {
				if matches!(
					state_update.state,
					protocol::ActorState::ActorStateStopped(_)
				) {
					// If the actor is being stopped by rivet, we don't need the entry anymore
					if entry.received_stop {
						remove_after_stop = true;
					}
				}
			}
		}
		if remove_after_stop {
			ctx.remove_actor(&event.checkpoint.actor_id, event.checkpoint.generation);
		}
	}

	// Send if connected
	ws_send(&ctx.shared, protocol::ToRivet::ToRivetEvents(events)).await;
}

pub fn handle_ack_events(ctx: &mut EnvoyContext, ack: protocol::ToEnvoyAckEvents) {
	for checkpoint in &ack.last_event_checkpoints {
		let key = (checkpoint.actor_id.to_string(), checkpoint.generation);
		if let Some(pending) = ctx.pending_events.get_mut(&key) {
			pending
				.events
				.retain(|event| event.checkpoint.index > checkpoint.index);
			if pending.events.is_empty() {
				ctx.pending_events.remove(&key);
			}
		}
	}
	if ctx.pending_events.is_empty() {
		for waiter in ctx.pending_events_ack_waiters.drain(..) {
			let _ = waiter.send(());
		}
	}
}

/// Marks a generation's unacknowledged events retired, keeping them for replay until
/// `TERMINAL_EVENT_RETENTION` passes. Called when the generation stops or leaves this
/// Envoy; the first call fixes the retirement time.
pub(crate) fn retire(ctx: &mut EnvoyContext, actor_id: &str, generation: u32) {
	let key = (actor_id.to_owned(), generation);
	let Some(pending) = ctx.pending_events.get_mut(&key) else {
		return;
	};
	if pending.retired_at.is_some() {
		return;
	}
	let now = crate::time::Instant::now();
	pending.retired_at = Some(now);
	ctx.deadlines.schedule(
		DeadlineKey::TerminalEvents(key.0, key.1),
		now + TERMINAL_EVENT_RETENTION,
	);
}

/// Discards a retired generation's events once their retention passed without an ack.
pub(crate) fn expire_terminal_events(
	ctx: &mut EnvoyContext,
	actor_id: String,
	generation: u32,
	now: crate::time::Instant,
) {
	let key = (actor_id, generation);
	let Some(retired_at) = ctx
		.pending_events
		.get(&key)
		.and_then(|pending| pending.retired_at)
	else {
		return;
	};
	if now.saturating_duration_since(retired_at) < TERMINAL_EVENT_RETENTION {
		ctx.deadlines.schedule(
			DeadlineKey::TerminalEvents(key.0, key.1),
			retired_at + TERMINAL_EVENT_RETENTION,
		);
		return;
	}
	ctx.pending_events.remove(&key);
	tracing::warn!(
		actor_id = %key.0,
		generation = key.1,
		"discarded expired terminal actor events"
	);
}

pub async fn resend_unacknowledged_events(ctx: &mut EnvoyContext) {
	crate::deadlines::expire_due(ctx);
	let mut events: Vec<protocol::EventWrapper> = Vec::new();

	for pending in ctx.pending_events.values() {
		events.extend(pending.events.iter().cloned());
	}

	if events.is_empty() {
		return;
	}

	tracing::info!(count = events.len(), "resending unacknowledged events");
	for event in &events {
		tracing::info!(event = %stringify_event_wrapper(event), "resending event");
	}

	ws_send(&ctx.shared, protocol::ToRivet::ToRivetEvents(events)).await;
}


