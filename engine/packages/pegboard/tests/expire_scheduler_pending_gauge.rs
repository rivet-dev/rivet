use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::{envoy_expire_scheduler::EnvoyExpireScheduler, metrics};
use tokio::sync::{mpsc, watch};

#[tokio::test]
async fn scheduler_pending_gauge_tracks_queued_work() -> Result<()> {
	let ns = Id::new_v1(1);
	let (started_tx, mut started_rx) = mpsc::unbounded_channel();
	let (release_tx, release_rx) = watch::channel(false);
	let scheduler = EnvoyExpireScheduler::new_for_tests(8, 1, move |_, _| {
		let started_tx = started_tx.clone();
		let mut release_rx = release_rx.clone();
		async move {
			let _ = started_tx.send(());
			release_rx
				.wait_for(|released| *released)
				.await
				.context("release watch closed")?;
			Ok(pegboard::ops::envoy::expire::Output { did_expire: false })
		}
	});

	scheduler.try_enqueue(ns, "gauge-envoy".to_string());
	started_rx.recv().await.context("worker did not start")?;
	assert!(metrics::ENVOY_EXPIRE_SCHEDULER_PENDING.get() >= 1);

	release_tx.send(true).context("release watch send failed")?;
	scheduler.wait_pending_empty().await;
	assert_eq!(metrics::ENVOY_EXPIRE_SCHEDULER_PENDING.get(), 0);

	Ok(())
}
