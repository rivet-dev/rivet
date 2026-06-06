use rivetkit::Actor;
use rivetkit::action::Raw;

/// Marker type implementing [`Actor`] for the agent-os actor.
///
/// Actions are dispatched by name in [`crate::run::run`] using
/// `action.decode_as::<(...)>()` with per-arm tuple types matching the
/// underlying `AgentOs::*` method signatures (TS sends positional args
/// as a CBOR array which decodes cleanly into a Rust tuple).
#[derive(Debug)]
pub struct AgentOsActor;

impl Actor for AgentOsActor {
	type Input = ();
	type ConnParams = serde_json::Value;
	type ConnState = ();
	type Action = Raw;
}
