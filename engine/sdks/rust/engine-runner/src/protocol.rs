use anyhow::Result;
use rivet_runner_protocol as rp;
use rivet_runner_protocol::mk2 as rp2;
use vbare::OwnedVersionedData;

pub const PROTOCOL_VERSION: u16 = rp::PROTOCOL_MK2_VERSION;

/// Helper to decode messages from server (MK2)
pub fn decode_to_client(buf: &[u8], protocol_version: u16) -> Result<rp2::ToClient> {
	// Use versioned deserialization to handle protocol version properly
	<rp::versioned::ToClientMk2 as OwnedVersionedData>::deserialize(buf, protocol_version)
}

/// Helper to encode messages to server (MK2)
pub fn encode_to_server(msg: rp2::ToServer) -> Vec<u8> {
	rp::versioned::ToServerMk2::wrap_latest(msg)
		.serialize(PROTOCOL_VERSION)
		.expect("failed to serialize ToServer")
}

/// Helper to create event wrapper with checkpoint (MK2)
pub fn make_event_wrapper(
	actor_id: &str,
	generation: u32,
	index: u64,
	event: rp2::Event,
) -> rp2::EventWrapper {
	rp2::EventWrapper {
		checkpoint: rp2::ActorCheckpoint {
			actor_id: actor_id.to_string(),
			generation,
			index: index as i64,
		},
		inner: event,
	}
}

/// Helper to create actor state update event (MK2)
pub fn make_actor_state_update(state: rp2::ActorState) -> rp2::Event {
	rp2::Event::EventActorStateUpdate(rp2::EventActorStateUpdate { state })
}

/// Helper to create actor intent event (MK2)
pub fn make_actor_intent(intent: rp2::ActorIntent) -> rp2::Event {
	rp2::Event::EventActorIntent(rp2::EventActorIntent { intent })
}

/// Helper to create set alarm event (MK2)
pub fn make_set_alarm(alarm_ts: Option<i64>) -> rp2::Event {
	rp2::Event::EventActorSetAlarm(rp2::EventActorSetAlarm { alarm_ts })
}
