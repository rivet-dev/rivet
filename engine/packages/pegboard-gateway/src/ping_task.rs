use anyhow::Result;
use rivet_runner_protocol as protocol;
use tokio::sync::watch;

use super::{LifecycleResult, UPDATE_PING_INTERVAL};
use crate::shared_state::SharedState;

pub async fn task(
	shared_state: SharedState,
	request_id: protocol::RequestId,
	mut ping_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		tokio::select! {
			_ = tokio::time::sleep(UPDATE_PING_INTERVAL) => {}
			_ = ping_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		shared_state.send_and_check_ping(request_id).await?;
	}
}
