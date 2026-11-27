//! Per-actor-type adapter settings: which hooks the host implements and how long
//! each may run. Defaults follow `rivetkit-napi` (`actor_factory.rs`).

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Hooks the host implements. A hook that is absent costs no round trip: the
/// adapter answers the event itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Hooks {
	pub on_migrate: bool,
	pub on_create: bool,
	pub on_fork: bool,
	pub on_wake: bool,
	pub on_sleep: bool,
	pub on_destroy: bool,
	pub on_before_connect: bool,
	pub on_connect: bool,
	pub on_disconnect: bool,
	pub on_subscribe: bool,
	pub on_request: bool,
	pub on_websocket: bool,
	/// The host wants a notification when core's abort signal fires (shutdown
	/// started), so it can stop background work.
	pub on_abort: bool,
}

#[derive(Clone, Debug)]
pub struct AdapterConfig {
	pub hooks: Hooks,
	pub on_migrate_timeout: Duration,
	/// The host's per-start setup (Java: the actor constructor).
	pub create_vars_timeout: Duration,
	pub on_create_timeout: Duration,
	pub on_wake_timeout: Duration,
	pub on_before_connect_timeout: Duration,
	pub on_connect_timeout: Duration,
	pub action_timeout: Duration,
	pub on_request_timeout: Duration,
}

impl AdapterConfig {
	/// napi defaults. `action_timeout` comes from core's actor config so both agree.
	pub fn new(hooks: Hooks, action_timeout: Duration) -> Self {
		Self {
			hooks,
			on_migrate_timeout: Duration::from_secs(30),
			create_vars_timeout: Duration::from_secs(5),
			on_create_timeout: Duration::from_secs(5),
			on_wake_timeout: Duration::from_secs(30),
			on_before_connect_timeout: Duration::from_secs(5),
			on_connect_timeout: Duration::from_secs(5),
			action_timeout,
			on_request_timeout: action_timeout,
		}
	}
}
