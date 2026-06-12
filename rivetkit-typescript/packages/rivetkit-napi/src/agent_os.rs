//! Phase 1b: NAPI binding for the agent-os actor.
//!
//! Exposes `NapiAgentOsOptions` (`#[napi(object)]`) and the
//! `NapiActorFactory::from_agent_os` static constructor. The constructor
//! parses the JSON-envelope config with `serde(deny_unknown_fields)` so
//! non-serializable or unknown fields fail loud at construction time,
//! then builds a `CoreActorFactory` via `rivetkit_agent_os::build_core_factory`.

use std::sync::Arc;

use agent_os_client::{AgentOsConfig, SoftwareInput};
use napi_derive::napi;
use rivetkit_agent_os::AgentOsActorConfig;

use crate::NapiInvalidArgument;
use crate::napi_anyhow_error;

#[napi(object)]
#[derive(Default)]
pub struct NapiAgentOsOptions {
	/// JSON-encoded subset of `AgentOsConfig`. Fields that cannot be
	/// represented in JSON (e.g. `schedule_driver`, `MountConfig::driver`)
	/// are intentionally absent; passing them in the JSON envelope must
	/// fail loud (enforced by `deny_unknown_fields`).
	pub config_json: Option<String>,
	/// Absolute path to the prebuilt `agent-os-sidecar` binary, resolved on
	/// the TypeScript side from the `@rivet-dev/agent-os-sidecar` npm package.
	/// Forwarded to the agent-os client via its `AGENT_OS_SIDECAR_BIN` env so
	/// the client spawns the bundled binary instead of relying on `PATH`.
	pub sidecar_binary_path: Option<String>,
}

/// Serializable mirror of [`AgentOsConfig`] for the Phase 1b minimal scope.
/// `deny_unknown_fields` enforces fail-loud behavior when callers pass
/// fields outside this allow-list (including non-serializable fields like
/// `schedule_driver` or `driver` on mounts).
#[derive(serde::Deserialize, Default, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentOsConfigJson {
	#[serde(default)]
	software: Vec<SoftwareInput>,
	#[serde(default)]
	additional_instructions: Option<String>,
	#[serde(default)]
	module_access_cwd: Option<String>,
	#[serde(default)]
	loopback_exempt_ports: Vec<u16>,
	#[serde(default)]
	allowed_node_builtins: Option<Vec<String>>,
}

impl AgentOsConfigJson {
	fn to_agent_os_config(&self) -> AgentOsConfig {
		AgentOsConfig {
			software: self.software.clone(),
			loopback_exempt_ports: self.loopback_exempt_ports.clone(),
			allowed_node_builtins: self.allowed_node_builtins.clone(),
			module_access_cwd: self.module_access_cwd.clone(),
			additional_instructions: self.additional_instructions.clone(),
			..AgentOsConfig::default()
		}
	}
}

/// Parse `NapiAgentOsOptions` into an `AgentOsActorConfig` whose builder
/// closure produces a fresh `AgentOsConfig` per actor instance (because
/// `AgentOsConfig` is non-`Clone`).
pub(crate) fn parse_agent_os_options(
	options: NapiAgentOsOptions,
) -> napi::Result<AgentOsActorConfig> {
	// Forward the npm-resolved sidecar binary path to the agent-os client. The
	// client reads `AGENT_OS_SIDECAR_BIN` when spawning the native sidecar, so
	// setting it here makes the bundled binary authoritative for this process.
	if let Some(path) = options.sidecar_binary_path.as_deref() {
		if !path.is_empty() {
			// SAFETY: runs once during factory construction at registry setup,
			// before any VM (and thus any agent-os client thread that reads this
			// var via `std::env::var`) is created. No other code reads
			// `AGENT_OS_SIDECAR_BIN` concurrently with this write.
			unsafe {
				std::env::set_var("AGENT_OS_SIDECAR_BIN", path);
			}
		}
	}

	let parsed: AgentOsConfigJson = match options.config_json.as_deref() {
		Some(json) => serde_json::from_str(json).map_err(|error| {
			napi_anyhow_error(
				NapiInvalidArgument {
					argument: "configJson".to_owned(),
					reason: format!("agent-os config JSON parse error: {error}"),
				}
				.build(),
			)
		})?,
		None => AgentOsConfigJson::default(),
	};
	let parsed = Arc::new(parsed);
	Ok(AgentOsActorConfig::from_builder(move || {
		parsed.to_agent_os_config()
	}))
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../tests/agent_os_factory.rs"]
mod tests;
