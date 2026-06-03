pub mod pubsub_subjects;

#[cfg(all(
	any(target_os = "linux", target_os = "macos"),
	any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub use pyroscope::start;

#[cfg(all(
	any(target_os = "linux", target_os = "macos"),
	any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod pyroscope;

#[cfg(not(all(
	any(target_os = "linux", target_os = "macos"),
	any(target_arch = "x86_64", target_arch = "aarch64")
)))]
#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> anyhow::Result<()> {
	drop(pools);

	if config.pyroscope.is_some() {
		tracing::warn!("pyroscope profiling is unsupported on this target, profiling disabled");
	} else {
		tracing::debug!("pyroscope not configured, profiling disabled");
	}

	std::future::pending::<()>().await;
	Ok(())
}
