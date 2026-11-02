//! Stand-in for `rivet_config::config::TabletBalancingConfig`, whose crate does not build
//! for wasm32. The visualization drives the production coordinator in the browser, so
//! the fields and defaults must match the engine config exactly.

/// Static tablet placement policy; membership and lease timings remain topology-owned.
#[derive(Debug, Clone)]
pub struct TabletBalancingConfig {
	pub enabled: bool,
	pub tablets_per_core: u32,
	pub destination_stability_ms: u64,
	pub missing_worker_ms: u64,
	pub max_disk_probes: u32,
}

impl Default for TabletBalancingConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			tablets_per_core: 2,
			destination_stability_ms: 60_000,
			missing_worker_ms: 120_000,
			max_disk_probes: 4,
		}
	}
}

impl TabletBalancingConfig {
	/// Unit conversion for the CPU capacity advertised by storage workers.
	pub const MILLICORES_PER_CORE: u32 = 1000;
}
