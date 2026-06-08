use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::envoy_expire_scheduler::EnvoyExpireScheduler;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};
use tokio::sync::mpsc;

#[tokio::test]
async fn scheduler_worker_panic_drops_pending_entry() -> Result<()> {
	let ns = Id::new_v1(1);
	let attempts = Arc::new(AtomicUsize::new(0));
	let (started_tx, mut started_rx) = mpsc::unbounded_channel();
	let scheduler = EnvoyExpireScheduler::new_for_tests(8, 1, {
		let attempts = Arc::clone(&attempts);
		move |_, _| {
			let attempts = Arc::clone(&attempts);
			let started_tx = started_tx.clone();
			async move {
				attempts.fetch_add(1, Ordering::SeqCst);
				let _ = started_tx.send(());
				panic!("forced scheduler worker panic");
			}
		}
	});

	scheduler.try_enqueue(ns, "panic-envoy".to_string());
	started_rx
		.recv()
		.await
		.context("first worker did not start")?;
	scheduler.wait_pending_empty().await;
	assert_eq!(scheduler.pending_len(), 0);

	scheduler.try_enqueue(ns, "panic-envoy".to_string());
	started_rx
		.recv()
		.await
		.context("second worker did not start")?;
	scheduler.wait_pending_empty().await;

	assert_eq!(attempts.load(Ordering::SeqCst), 2);

	Ok(())
}
