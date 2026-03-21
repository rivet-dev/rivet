use anyhow::Result;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION};
use vbare::OwnedVersionedData;

/// Helper to decode messages from rivet
pub fn decode_to_envoy(buf: &[u8], protocol_version: u16) -> Result<protocol::ToEnvoy> {
	// Use versioned deserialization to handle protocol version properly
	<protocol::versioned::ToEnvoy as OwnedVersionedData>::deserialize(buf, protocol_version)
}

/// Helper to encode messages to rivet
pub fn encode_to_rivet(msg: protocol::ToRivet) -> Vec<u8> {
	protocol::versioned::ToRivet::wrap_latest(msg)
		.serialize(PROTOCOL_VERSION)
		.expect("failed to serialize ToRivet")
}

/// Helper to create event wrapper with checkpoint
pub fn make_event_wrapper(
	actor_id: &str,
	generation: u32,
	index: u64,
	event: protocol::Event,
) -> protocol::EventWrapper {
	protocol::EventWrapper {
		checkpoint: protocol::ActorCheckpoint {
			actor_id: actor_id.to_string(),
			generation,
			index: index as i64,
		},
		inner: event,
	}
}

/// Helper to create actor state update event
pub fn make_actor_state_update(state: protocol::ActorState) -> protocol::Event {
	protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate { state })
}

/// Helper to create actor intent event
pub fn make_actor_intent(intent: protocol::ActorIntent) -> protocol::Event {
	protocol::Event::EventActorIntent(protocol::EventActorIntent { intent })
}

/// Helper to create set alarm event
pub fn make_set_alarm(alarm_ts: Option<i64>) -> protocol::Event {
	protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm { alarm_ts })
}
