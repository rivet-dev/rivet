use std::collections::HashMap;

use rivetkit::Actor;
use rivetkit::action::Raw;
use tokio::task::JoinHandle;

/// Marker type implementing [`Actor`] for the agent-os actor.
///
/// Actions are dispatched by name in [`crate::run::run`] using
/// `action.decode_as::<(...)>()` with per-arm tuple types matching the
/// underlying `AgentOs::*` method signatures (TS sends positional args
/// as a CBOR array which decodes cleanly into a Rust tuple).
#[derive(Debug)]
pub struct AgentOsActor;

impl Actor for AgentOsActor {
	type State = ();
	type Input = ();
	type Actions = ();
	type Events = ();
	type Queue = ();
	type ConnParams = serde_json::Value;
	type ConnState = ();
	type Action = Raw;

	// Agent-os persists all of its state (filesystem, sessions, previews) to the
	// actor's SQLite database via `ctx.sql()`, so the actor needs a database.
	const HAS_DATABASE: bool = true;
}

/// Ephemeral per-VM-lifetime actor state (session-resume, spec §3/§5/§8).
///
/// Everything here is reconstructed on each wake from the durable SQLite tables
/// and the freshly created VM — it is intentionally NOT persisted:
///
/// - `live_sessions` is the `external_session_id -> live_session_id` remap. It
///   lives SOLELY in the actor (the sidecar is stateless across VM lifetimes and
///   only ever knows live ids). For native `session/load` resume `external ==
///   live`; for the fallback `session/new` tier the agent assigns a new id and
///   the actor records `external -> live` here. The client never sees `live`.
/// - `capture_tasks` holds the spawned `on_session_event` pump task per live
///   session, so capture can be cancelled on close / sleep / destroy. The
///   subscription is broadcast-backed, so aborting the task (which drops the
///   stream) is the unsubscribe.
#[derive(Default)]
pub struct Vars {
	/// `external_session_id -> live_session_id`.
	pub live_sessions: HashMap<String, String>,
	/// `live_session_id -> capture pump task`.
	pub capture_tasks: HashMap<String, JoinHandle<()>>,
}

impl Vars {
	/// Resolve a client-facing `external_session_id` to the live ACP session id,
	/// falling back to the external id itself (the native / not-yet-resumed case
	/// where `external == live`).
	pub fn live_id<'a>(&'a self, external_session_id: &'a str) -> &'a str {
		self.live_sessions
			.get(external_session_id)
			.map(String::as_str)
			.unwrap_or(external_session_id)
	}

	/// Abort and clear all in-flight capture tasks. Called on VM teardown
	/// (sleep / destroy / run-loop exit); the spawned task dropping its event
	/// stream is the unsubscribe.
	pub fn clear(&mut self) {
		for (_, task) in self.capture_tasks.drain() {
			task.abort();
		}
		self.live_sessions.clear();
	}
}
