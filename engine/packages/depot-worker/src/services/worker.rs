//! Depot storage worker process service.

use crate::{Worker, error::Result};
use rivet_topology::topology::Topology;
use std::sync::Arc;
use universalpubsub::PubSub;

pub async fn start(
	mut term_signal: rivet_env::capabilities::shutdown::Shutdown,
	config: rivet_config::Config,
	pubsub: PubSub,
	topology: Arc<Topology>,
) -> Result<()> {
	let env = pubsub.env().clone();
	let clock = env.clock();

	let worker = Worker::start(&config, pubsub.clone(), topology).await?;

	term_signal.recv().await?;
	let deadline = term_signal.deadline(config.runtime.force_shutdown_duration())?;

	// UPS owns process draining. Keep membership alive until accepted requests
	// finish; returning then allows UPS to finish shutting down.
	pubsub.requests_drained().await;
	rivet_env::timeout_at(clock.as_ref(), deadline, worker.close()).await??;

	Ok(())
}
