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
