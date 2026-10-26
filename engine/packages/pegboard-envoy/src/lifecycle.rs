//! Translates envoy lifecycle events into actor state machine events.
use anyhow::{Context, Result};
use pegboard::{keys::partition::ActorVolume, wire};
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as p;
use rivet_util_id::Id;
use std::{future::Future, time::Duration};

/// Maps one envoy event to the actor event it drives. The envoy protocol's generation is the
/// actor's epoch.
pub(crate) fn actor_event(event: &p::EventWrapper) -> wire::ActorEvent {
	let kind = match &event.inner {
		p::Event::EventActorIntent(intent) => match intent.intent {
			p::ActorIntent::ActorIntentSleep => wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent,
			p::ActorIntent::ActorIntentStop => wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent,
		},
		p::Event::EventActorStateUpdate(update) => match &update.state {
			p::ActorState::ActorStateRunning => wire::ToEnvoyEventKind::ToEnvoyEventKindRunning,
			p::ActorState::ActorStateStopped(stopped) => {
				wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(wire::ToEnvoyEventKindStopped {
					ok: match stopped.code {
						p::StopCode::Ok => true,
						p::StopCode::Error => false,
					},
					message: stopped
						.message
						.as_deref()
						.map(truncate_message)
						.map(Into::into),
				})
			}
		},
		p::Event::EventActorSetAlarm(alarm) => {
			wire::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(wire::ToEnvoyEventKindSetAlarm {
				alarm_unix_ms: alarm.alarm_ts,
			})
		}
	};

	wire::ActorEvent::ToEnvoyEvent(wire::ToEnvoyEvent {
		epoch: event.checkpoint.generation,
		kind,
	})
}

/// The longest stop message kept in the actor's state, in bytes.
const MAX_STOP_MESSAGE_BYTES: usize = 1024;

/// Truncates a stop message to [`MAX_STOP_MESSAGE_BYTES`] on a character boundary, so a large
/// error string cannot grow every later actor state write.
pub(crate) fn truncate_message(message: &str) -> String {
	let mut end = message.len().min(MAX_STOP_MESSAGE_BYTES);
	while !message.is_char_boundary(end) {
		end -= 1;
	}
	message[..end].to_owned()
}

/// Whether the event reports that its epoch stopped.
pub(crate) fn is_stopped(event: &p::EventWrapper) -> bool {
	matches!(
		&event.inner,
		p::Event::EventActorStateUpdate(p::EventActorStateUpdate {
			state: p::ActorState::ActorStateStopped(_)
		})
	)
}

/// Delivers one envoy event to the actor state machine. Returns `false` when the delivery
/// budget ran out, leaving the event unacknowledged so the envoy replays it after reconnecting.
///
/// Replaying is safe because the state machine applies an envoy event only to the epoch it
/// names and only in the phase it expects, so a second delivery of a committed event is a no-op.
pub(crate) async fn deliver(
	ctx: &EngineContext,
	actor_id: &str,
	event: p::EventWrapper,
) -> Result<bool> {
	let actor_event = actor_event(&event);
	let id = Id::parse(actor_id).context("invalid envoy event actor id")?;
	let volume = ActorVolume::new(id).volume_key();
	let budget = Duration::from_millis(ctx.config().pegboard().envoy_event_delivery_timeout_ms());

	let depot = ctx.depot();
	let delivered = retry_within(ctx.env().as_ref(), budget, || async {
		depot
			.command::<pegboard::commands::ActorEvent>(&volume, actor_event.clone())
			.await
			.map(|_response| ())
	})
	.await;

	match delivered {
		Ok(()) => Ok(true),
		Err(error) => {
			tracing::warn!(
				%actor_id,
				epoch = event.checkpoint.generation,
				index = event.checkpoint.index,
				?error,
				"actor event delivery exhausted its budget; storage may be overloaded"
			);
			Ok(false)
		}
	}
}

/// Retries `send` with capped exponential backoff until it succeeds or `budget` elapses.
pub(crate) async fn retry_within<T, F, Fut>(
	env: &dyn rivet_env::Env,
	budget: Duration,
	mut send: F,
) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<T>>,
{
	let clock = env.clock();
	rivet_env::timeout(clock.as_ref(), budget, async {
		let mut delay = Duration::from_millis(10);
		loop {
			match send().await {
				Ok(value) => return Ok(value),
				Err(error) => {
					tracing::debug!(?error, "actor event command failed; retrying");
					rivet_env::sleep(clock.as_ref(), delay).await?;
					delay = (delay * 2).min(Duration::from_millis(500));
				}
			}
		}
	})
	.await?
}


