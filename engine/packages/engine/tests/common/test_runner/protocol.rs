use anyhow::*;
use rivet_runner_protocol as rp;
use vbare::OwnedVersionedData;

pub const PROTOCOL_VERSION: u16 = rp::PROTOCOL_MK1_VERSION;

/// Helper to decode messages from server
pub fn decode_to_client(buf: &[u8], protocol_version: u16) -> Result<rp::ToClient> {
	// Use versioned deserialization to handle protocol version properly
	<rp::versioned::ToClient as OwnedVersionedData>::deserialize(buf, protocol_version)
}

/// Helper to encode messages to server
pub fn encode_to_server(msg: rp::ToServer) -> Vec<u8> {
	rp::versioned::ToServer::wrap_latest(msg)
		.serialize(PROTOCOL_VERSION)
		.expect("failed to serialize ToServer")
}

/// Helper to create event wrapper with index
pub fn make_event_wrapper(index: u64, event: rp::Event) -> rp::EventWrapper {
	rp::EventWrapper {
		index: index as i64,
		inner: event,
	}
}

/// Helper to create actor state update event
pub fn make_actor_state_update(
	actor_id: &str,
	generation: u32,
	state: rp::ActorState,
) -> rp::Event {
	rp::Event::EventActorStateUpdate(rp::EventActorStateUpdate {
		actor_id: actor_id.to_string(),
		generation,
		state,
	})
}

/// Helper to create actor intent event
pub fn make_actor_intent(actor_id: &str, generation: u32, intent: rp::ActorIntent) -> rp::Event {
	rp::Event::EventActorIntent(rp::EventActorIntent {
		actor_id: actor_id.to_string(),
		generation,
		intent,
	})
}

/// Helper to create set alarm event
pub fn make_set_alarm(actor_id: &str, generation: u32, alarm_ts: Option<i64>) -> rp::Event {
	rp::Event::EventActorSetAlarm(rp::EventActorSetAlarm {
		actor_id: actor_id.to_string(),
		generation,
		alarm_ts,
	})
}
