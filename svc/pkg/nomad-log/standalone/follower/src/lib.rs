mod alloc_monitor;
mod log_shipper;
mod task_follower;

use rivet_operation::prelude::*;
use tokio::sync::{broadcast, mpsc};

#[tracing::instrument(skip_all)]
pub async fn start(shared_client: chirp_client::SharedClientHandle) -> GlobalResult<()> {
	let client = shared_client.clone().wrap_new("nomad-log-follower");

	let nomad_config = nomad_util::config_from_env()?;

	let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(8);
	let (log_tx, log_rx) = mpsc::channel(1024);

	// Build log shipper
	let log_shipper = log_shipper::LogShipper::new(
		client,
		(shutdown_tx.clone(), shutdown_tx.subscribe()),
		log_rx,
	);

	// Build alloc monitor
	let alloc_monitor = alloc_monitor::AllocMonitor::new(
		nomad_config,
		(shutdown_tx.clone(), shutdown_tx.subscribe()),
		log_tx,
		Box::new(|job_id| util_job::is_nomad_job_run(job_id) || job_id.starts_with("job-test-")),
	)
	.await?;

	// Spawn tasks
	tokio::task::Builder::new()
		.name("nomad_log_follower::log_shipper")
		.spawn(log_shipper.start())?;
	tokio::task::Builder::new()
		.name("nomad_log_follower::alloc_monitor")
		.spawn(alloc_monitor.start())?;

	// Wait for shutdown
	tokio::select! {
		res = tokio::signal::ctrl_c() => {
			if let Err(err) = res {
				tracing::error!(?err, "err in ctrl c");
			}

			tracing::info!("received ctrl c");
		}
		res = shutdown_rx.recv() => {
			tracing::info!("main thread received shutdown");
			if let Err(err) = res {
				tracing::error!(?err, "err in shutdown recv");
			}
		}
	}

	tracing::info!("main thread shutting down");
	shutdown_tx.send(())?;

	Ok(())
}
