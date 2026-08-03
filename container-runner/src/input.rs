//! The actor input payload describing how to launch the child game server, and
//! the persisted actor state that wraps it.
//!
//! Everything the game server needs to launch (command, args, env, port) is
//! carried in the actor's create-time `input` payload, CBOR-encoded per the
//! RivetKit convention. All fields are optional; anything omitted falls back
//! to the CLI-provided template (`rivet-container-runner -- <command...>`).
//! The input is preserved inside `ActorState` so a woken actor restores the
//! same launch spec without re-decoding input.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Shape of the actor `input` payload. Unknown fields are ignored rather than
/// rejected: a strict decode would break waking actors after a rollback to a
/// binary that predates a newly added field.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ActorInput {
	/// Overrides the CLI command template entirely (program + fixed args).
	#[serde(default)]
	pub command: Option<Vec<String>>,
	/// Extra args appended after the command template / `command`.
	#[serde(default)]
	pub args: Vec<String>,
	/// Extra environment variables for the child process.
	#[serde(default)]
	pub env: HashMap<String, String>,
	/// Local port the child listens on; also exported to the child as `PORT`.
	/// Falls back to the runner's `--child-port` when omitted.
	#[serde(default)]
	pub port: Option<u16>,
}

/// Persisted actor state. Wraps the launch spec and tracks whether the container
/// has already started once. Unknown fields are ignored for the same
/// rollback-safety reason as `ActorInput`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ActorState {
	/// The create-time launch spec, preserved so a woken actor can respawn the
	/// same child without the engine re-sending input.
	#[serde(default)]
	pub input: ActorInput,
	/// `false` on create, set to `true` on the container's first start and
	/// persisted. A container hosts exactly one child lifetime: seeing this set on
	/// a later start means a restart after the container stopped, so the woken
	/// actor destroys itself instead of respawning. Tracked on start rather than
	/// on stop because startup writes persist reliably while an `on_sleep` write
	/// races the shutdown state serialization.
	#[serde(default)]
	pub did_start: bool,
}

#[cfg(test)]
#[path = "../tests/inline/input.rs"]
mod tests;
