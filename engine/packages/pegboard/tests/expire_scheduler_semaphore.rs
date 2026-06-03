use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::envoy_expire_scheduler::EnvoyExpireScheduler;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, watch};

#[tokio::test]
async fn scheduler_semaphore_bounds_in_flight_workers() -> Result<()> {
	let ns = Id::new_v1(1);
	let current = Arc::new(AtomicUsize::new(0));
	let max_seen = Arc::new(AtomicUsize::new(0));
	let (started_tx, mut started_rx) = mpsc::unbounded_channel();
	let (done_tx, mut done_rx) = mpsc::unbounded_channel();
	let (release_tx, release_rx) = watch::channel(false);
	let scheduler = EnvoyExpireScheduler::new_for_tests(32, 2, {
		let current = Arc::clone(&current);
		let max_seen = Arc::clone(&max_seen);
		move |_, _| {
			let current = Arc::clone(&current);
			let max_seen = Arc::clone(&max_seen);
			let started_tx = started_tx.clone();
			let done_tx = done_tx.clone();
			let mut release_rx = release_rx.clone();
			async move {
				let in_flight = current.fetch_add(1, Ordering::SeqCst) + 1;
				max_seen.fetch_max(in_flight, Ordering::SeqCst);
				let _ = started_tx.send(());
				release_rx
					.wait_for(|released| *released)
					.await
					.context("release watch closed")?;
				current.fetch_sub(1, Ordering::SeqCst);
				let _ = done_tx.send(());
				Ok(pegboard::ops::envoy::expire::Output { did_expire: false })
			}
		}
	});

	for i in 0..10 {
		scheduler.try_enqueue(ns, format!("envoy-{i}"));
	}

	started_rx
		.recv()
		.await
		.context("first worker did not start")?;
	started_rx
		.recv()
		.await
		.context("second worker did not start")?;
	assert_eq!(current.load(Ordering::SeqCst), 2);

	release_tx.send(true).context("release watch send failed")?;
	for _ in 0..10 {
		done_rx.recv().await.context("worker did not finish")?;
	}
	scheduler.wait_pending_empty().await;

	assert!(max_seen.load(Ordering::SeqCst) <= 2);

	Ok(())
}
