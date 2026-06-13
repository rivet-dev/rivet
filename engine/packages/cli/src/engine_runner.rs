use std::{
	env,
	path::{Path, PathBuf},
};

use rivetkit_engine_process::EngineResolverConfig;

use crate::DEFAULT_ENGINE_ENDPOINT;

/// Builds the engine resolver config shared by `rivet dev` and `rivet engine`.
///
/// Resolution order (handled by the engine-process crate): the explicit
/// `--engine-binary` path, then `RIVET_ENGINE_BINARY_PATH`, then a binary
/// bundled next to this CLI, then a local build, then an auto-downloaded
/// release.
pub fn engine_config(engine_binary: Option<PathBuf>) -> EngineResolverConfig {
	let explicit = engine_binary.or_else(|| {
		let bundled = bundled_engine_binary();
		bundled.exists().then_some(bundled)
	});

	EngineResolverConfig::from_parts(
		DEFAULT_ENGINE_ENDPOINT,
		explicit,
		None,
		None,
		engine_auto_download(),
	)
}

/// Whether the CLI may download a release engine binary when none is found
/// locally. Enabled by default for the CLI; set `RIVETKIT_ENGINE_AUTO_DOWNLOAD`
/// to `0` or `false` to require a local binary.
fn engine_auto_download() -> bool {
	match env::var("RIVETKIT_ENGINE_AUTO_DOWNLOAD") {
		Ok(value) => !matches!(value.trim(), "0" | "false" | ""),
		Err(_) => true,
	}
}

/// Path to a rivet-engine binary distributed next to this CLI binary.
fn bundled_engine_binary() -> PathBuf {
	let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("rivet"));
	let name = if cfg!(windows) {
		"rivet-engine.exe"
	} else {
		"rivet-engine"
	};
	exe.parent().unwrap_or_else(|| Path::new(".")).join(name)
}
