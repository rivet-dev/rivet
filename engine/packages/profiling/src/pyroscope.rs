use ::pyroscope::PyroscopeAgent;
use ::pyroscope::backend::{BackendConfig, PprofConfig, pprof_backend};
use ::pyroscope::pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentRunning};
use anyhow::Result;
use gas::prelude::*;
use universalpubsub::NextOutput;

use crate::pubsub_subjects::{
	PROFILE_CONFIG_SUBJECT, ProfileConfigSubject, SetProfileConfigMessage,
};

/// Listens for runtime profiling toggles and owns the Pyroscope agent lifecycle.
///
/// The agent starts off and is enabled/disabled at runtime via UPS broadcasts (driven by the
/// `profile enable`/`profile disable` CLI). Requires `config.pyroscope` to be set; without it the
/// service is a no-op.
#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let Some(pyroscope) = config.pyroscope.clone() else {
		// Park forever rather than returning: this is a Core service that is always scheduled, and
		// returning would make the service manager treat it as an unexpected exit and restart-loop.
		tracing::debug!("pyroscope not configured, profiling disabled");
		std::future::pending::<()>().await;
		return Ok(());
	};

	let server_url = pyroscope.server_url.clone();
	let default_sample_rate = pyroscope.sample_rate();

	// Profile tags. The datacenter comes from the config; the pod name from the downward-API env.
	// Empty values are dropped so the tag set stays clean outside k8s.
	let dc = config.dc_name().map(|dc| dc.to_string())?;
	let pod = std::env::var("K8S_POD_NAME").unwrap_or_else(|_| "unknown".to_string());

	// Subscribe to profiling config updates.
	let ups = pools.ups()?;
	let mut sub = ups.subscribe(ProfileConfigSubject).await?;

	tracing::debug!(subject = %PROFILE_CONFIG_SUBJECT, server_url = %server_url, "subscribed to profile config updates");

	let mut running_agent: Option<PyroscopeAgent<PyroscopeAgentRunning>> = None;

	while let Ok(NextOutput::Message(msg)) = sub.next().await {
		match serde_json::from_slice::<SetProfileConfigMessage>(&msg.payload) {
			Ok(update) => {
				if update.enabled {
					if running_agent.is_some() {
						tracing::debug!("profiler already running");
						continue;
					}

					let sample_rate = update.sample_rate.unwrap_or(default_sample_rate);
					match start_agent(&server_url, sample_rate, &dc, &pod) {
						Ok(agent) => {
							running_agent = Some(agent);
							tracing::info!(server_url = %server_url, sample_rate, "profiler started");
						}
						Err(err) => tracing::error!(?err, "failed to start profiler"),
					}
				} else {
					let Some(agent) = running_agent.take() else {
						tracing::debug!("profiler already stopped");
						continue;
					};

					match agent.stop() {
						Ok(ready) => {
							ready.shutdown();
							tracing::info!("profiler stopped");
						}
						Err(err) => tracing::error!(?err, "failed to stop profiler"),
					}
				}
			}
			Err(err) => {
				tracing::error!(?err, "failed to deserialize profile config update message");
			}
		}
	}

	Ok(())
}

/// Builds and starts a Pyroscope agent that pushes pprof CPU profiles to the configured server.
fn start_agent(
	server_url: &str,
	sample_rate: u32,
	dc: &str,
	pod: &str,
) -> Result<PyroscopeAgent<PyroscopeAgentRunning>> {
	let backend = pprof_backend(PprofConfig { sample_rate }, BackendConfig::default());

	let agent = PyroscopeAgentBuilder::new(
		server_url,
		"rivet-engine",
		sample_rate,
		"rivet-engine",
		env!("CARGO_PKG_VERSION"),
		backend,
	)
	.tags(vec![("dc", dc), ("pod", pod)])
	.build()?;

	Ok(agent.start()?)
}
