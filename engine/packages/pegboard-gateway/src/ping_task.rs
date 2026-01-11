use anyhow::Result;
use rand::Rng;
use rivet_runner_protocol as protocol;
use std::time::Duration;
use tokio::sync::watch;

use super::LifecycleResult;
use crate::shared_state::SharedState;

pub async fn task(
	shared_state: SharedState,
	request_id: protocol::RequestId,
	mut ping_abort_rx: watch::Receiver<()>,
	update_ping_interval: Duration,
) -> Result<LifecycleResult> {
	loop {
		tokio::select! {
			_ = tokio::time::sleep(update_ping_interval) => {}
			_ = ping_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		// Jitter sleep to prevent stampeding herds
		let jitter = { rand::thread_rng().gen_range(0..128) };
		tokio::time::sleep(Duration::from_millis(jitter)).await;

		shared_state.send_and_check_ping(request_id).await?;
	}
}
