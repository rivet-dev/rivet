use gas::prelude::*;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use std::sync::{Arc, atomic::Ordering};
use tokio::sync::watch;

use crate::{LifecycleResult, UPDATE_PING_INTERVAL, conn::Conn};

/// Updates the ping of all runners requesting a ping update at once.
#[tracing::instrument(skip_all)]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut ping_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		tokio::select! {
			_ = tokio::time::sleep(UPDATE_PING_INTERVAL) => {}
			_ = ping_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		update_runner_ping(&ctx, &conn).await?;
	}
}

async fn update_runner_ping(ctx: &StandaloneCtx, conn: &Conn) -> Result<()> {
	let Some(wf) = ctx
		.workflow::<pegboard::workflows::runner::Input>(conn.workflow_id)
		.get()
		.await?
	else {
		tracing::error!(?conn.runner_id, "workflow does not exist");
		return Ok(());
	};

	// Check workflow is not dead
	if !wf.has_wake_condition {
		return Ok(());
	}

	// Update ping
	let rtt = conn.last_rtt.load(Ordering::Relaxed);
	let res = ctx
		.op(pegboard::ops::runner::update_alloc_idx::Input {
			runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
				runner_id: conn.runner_id,
				action: Action::UpdatePing { rtt },
			}],
		})
		.await?;

	// If runner became eligible again, then pull any pending actors
	for notif in res.notifications {
		if let RunnerEligibility::ReEligible = notif.eligibility {
			tracing::debug!(runner_id=?notif.runner_id, "runner has become eligible again");

			ctx.signal(pegboard::workflows::runner::CheckQueue {})
				.to_workflow_id(notif.workflow_id)
				.send()
				.await?;
		}
	}

	Ok(())
}
