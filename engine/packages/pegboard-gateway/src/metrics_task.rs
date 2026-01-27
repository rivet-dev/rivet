use std::sync::{Arc, atomic::AtomicU64};

use anyhow::Result;
use gas::prelude::*;
use tokio::sync::watch;

use super::{LifecycleResult, Metric, UPDATE_METRICS_INTERVAL, record_req_metrics};

pub async fn task(
	ctx: StandaloneCtx,
	actor_id: Id,
	runner_id: Id,
	ingress_bytes: Arc<AtomicU64>,
	egress_bytes: Arc<AtomicU64>,
	mut metrics_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut last_ingress_bytes = 0;
	let mut last_egress_bytes = 0;

	loop {
		tokio::select! {
			_ = tokio::time::sleep(UPDATE_METRICS_INTERVAL) => {}
			_ = metrics_abort_rx.changed() => {
				// Record final values before abort
				record_ws_transfer(
					&ctx,
					actor_id,
					runner_id,
					&ingress_bytes,
					&egress_bytes,
					&mut last_ingress_bytes,
					&mut last_egress_bytes,
				).await?;

				return Ok(LifecycleResult::Aborted);
			}
		}

		record_ws_transfer(
			&ctx,
			actor_id,
			runner_id,
			&ingress_bytes,
			&egress_bytes,
			&mut last_ingress_bytes,
			&mut last_egress_bytes,
		)
		.await?;
	}
}

pub async fn record_ws_transfer(
	ctx: &StandaloneCtx,
	actor_id: Id,
	runner_id: Id,
	ingress_bytes: &AtomicU64,
	egress_bytes: &AtomicU64,
	last_ingress_bytes: &mut u64,
	last_egress_bytes: &mut u64,
) -> Result<()> {
	let new_ingress_bytes = ingress_bytes.load(std::sync::atomic::Ordering::Acquire);
	let new_egress_bytes = egress_bytes.load(std::sync::atomic::Ordering::Acquire);
	let ingress_diff = new_ingress_bytes - *last_ingress_bytes;
	let egress_diff = new_egress_bytes - *last_egress_bytes;

	if ingress_diff > 0 || egress_diff > 0 {
		record_req_metrics(
			ctx,
			runner_id,
			actor_id,
			Metric::WebsocketTransfer(ingress_diff as usize, egress_diff as usize),
		)
		.await?;
	}

	*last_ingress_bytes = new_ingress_bytes;
	*last_egress_bytes = new_egress_bytes;

	Ok(())
}
