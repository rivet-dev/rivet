use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::{envoy_expire_scheduler::EnvoyExpireScheduler, metrics};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};
use tokio::{
	sync::{mpsc, watch},
	task::JoinSet,
};

#[tokio::test]
async fn concurrent_enqueues_for_one_envoy_dedupe_to_one_worker() -> Result<()> {
	let ns = Id::new_v1(1);
	let namespace_id = ns.to_string();
	let before = metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
		.with_label_values(&[namespace_id.as_str(), "deduped"])
		.get();
	let spawn_count = Arc::new(AtomicUsize::new(0));
	let (started_tx, mut started_rx) = mpsc::unbounded_channel();
	let (release_tx, release_rx) = watch::channel(false);
	let scheduler = EnvoyExpireScheduler::new_for_tests(1024, 1, {
		let spawn_count = Arc::clone(&spawn_count);
		move |_, _| {
			let spawn_count = Arc::clone(&spawn_count);
			let started_tx = started_tx.clone();
			let mut release_rx = release_rx.clone();
			async move {
				spawn_count.fetch_add(1, Ordering::SeqCst);
				let _ = started_tx.send(());
				release_rx
					.wait_for(|released| *released)
					.await
					.context("release watch closed")?;
				Ok(pegboard::ops::envoy::expire::Output { did_expire: false })
			}
		}
	});

	let mut tasks = JoinSet::new();
	for _ in 0..100 {
		let scheduler = Arc::clone(&scheduler);
		tasks.spawn(async move {
			scheduler.try_enqueue(ns, "envoy-a".to_string());
		});
	}
	while let Some(result) = tasks.join_next().await {
		result?;
	}

	started_rx.recv().await.context("worker did not start")?;
	assert_eq!(spawn_count.load(Ordering::SeqCst), 1);

	let after = metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
		.with_label_values(&[namespace_id.as_str(), "deduped"])
		.get();
	assert_eq!(after - before, 99);

	release_tx.send(true).context("release watch send failed")?;
	scheduler.wait_pending_empty().await;

	Ok(())
}
