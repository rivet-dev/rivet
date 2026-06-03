use anyhow::{Context, Result};
use gas::prelude::*;
use pegboard::{envoy_expire_scheduler::EnvoyExpireScheduler, metrics};
use tokio::sync::watch;

#[tokio::test]
async fn scheduler_rejects_when_pending_capacity_is_full() -> Result<()> {
	let ns = Id::new_v1(1);
	let namespace_id = ns.to_string();
	let before = metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
		.with_label_values(&[namespace_id.as_str(), "rejected_capacity"])
		.get();
	let (release_tx, release_rx) = watch::channel(false);
	let scheduler = EnvoyExpireScheduler::new_for_tests(4, 1, move |_, _| {
		let mut release_rx = release_rx.clone();
		async move {
			release_rx
				.wait_for(|released| *released)
				.await
				.context("release watch closed")?;
			Ok(pegboard::ops::envoy::expire::Output { did_expire: false })
		}
	});

	for i in 0..100 {
		scheduler.try_enqueue(ns, format!("envoy-{i}"));
		assert!(scheduler.pending_len() <= 4);
	}

	let after = metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
		.with_label_values(&[namespace_id.as_str(), "rejected_capacity"])
		.get();
	assert!(after > before);

	release_tx.send(true).context("release watch send failed")?;
	scheduler.wait_pending_empty().await;

	Ok(())
}
