//! Build-time metadata stamped into the binary by `build.rs`.
//!
//! This lives in its own crate because the git SHA and build timestamp change on every commit, and
//! `build.rs` declares them as `rerun-if-env-changed`. Anything that depends on this crate
//! therefore recompiles on every new commit, so only the binary should depend on it. Everything
//! else reads the values off `rivet_config::Config::build_meta` after [`publish`] stamps them.

/// Runtime identifier for the engine
pub const RUNTIME: &str = "engine";

/// Package version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit SHA
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");

/// Build timestamp
pub const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");

/// Rustc version used to compile
pub const RUSTC_VERSION: &str = env!("VERGEN_RUSTC_SEMVER");

/// Rustc host triple
pub const RUSTC_HOST: &str = env!("VERGEN_RUSTC_HOST_TRIPLE");

/// Cargo target triple
pub const CARGO_TARGET: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");

/// Cargo debug flag as string
const CARGO_DEBUG: &str = env!("VERGEN_CARGO_DEBUG");

/// Cargo profile (debug or release)
/// Returns "debug" if VERGEN_CARGO_DEBUG is "true", otherwise "release"
pub fn cargo_profile() -> &'static str {
	if CARGO_DEBUG == "true" {
		"debug"
	} else {
		"release"
	}
}

pub fn build_meta() -> rivet_config::BuildMeta {
	rivet_config::BuildMeta {
		runtime: RUNTIME.to_string(),
		version: VERSION.to_string(),
		git_sha: GIT_SHA.to_string(),
		build_timestamp: BUILD_TIMESTAMP.to_string(),
		rustc_version: RUSTC_VERSION.to_string(),
		rustc_host: RUSTC_HOST.to_string(),
		cargo_target: CARGO_TARGET.to_string(),
		cargo_profile: cargo_profile().to_string(),
	}
}

// NOTE: This is defined here instead of in the config crate to reduce the amount of recompilation should any
// of these change.
pub fn compiled_runtime_protocols() -> rivet_config::RuntimeProtocols {
	rivet_config::RuntimeProtocols {
		envoy: rivet_config::RuntimeProtocol::new(
			rivet_config::RuntimeProtocolKind::Envoy,
			rivet_envoy_protocol::PROTOCOL_VERSION,
		),
		ups: rivet_config::RuntimeProtocol::new(
			rivet_config::RuntimeProtocolKind::Ups,
			rivet_ups_protocol::PROTOCOL_VERSION,
		),
		epoxy: rivet_config::RuntimeProtocol::new(
			rivet_config::RuntimeProtocolKind::Epoxy,
			epoxy_protocol::PROTOCOL_VERSION,
		),
	}
}

pub fn pretty_print() -> String {
	format!(
		"{}\nGit SHA: {}\nBuild Timestamp: {}\nRustc Version: {}\nRustc Host: {}\nCargo Target: {}\nCargo Profile: {}",
		VERSION,
		GIT_SHA,
		BUILD_TIMESTAMP,
		RUSTC_VERSION,
		RUSTC_HOST,
		CARGO_TARGET,
		cargo_profile()
	)
}
