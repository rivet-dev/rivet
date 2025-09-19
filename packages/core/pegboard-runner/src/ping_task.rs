use gas::prelude::*;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use std::sync::{Arc, atomic::Ordering};

use crate::{UPDATE_PING_INTERVAL, conn::Conn};

pub async fn task(ctx: StandaloneCtx, conn: Arc<Conn>) {
	match task_inner(ctx, conn).await {
		Ok(_) => {}
		Err(err) => {
			tracing::error!(?err, "ping task errored");
		}
	}
}

/// Updates the ping of all runners requesting a ping update at once.
#[tracing::instrument(skip_all)]
async fn task_inner(ctx: StandaloneCtx, conn: Arc<Conn>) -> Result<()> {
	loop {
		tokio::time::sleep(UPDATE_PING_INTERVAL).await;

		// Check that workflow is not dead
		let Some(wf) = ctx
			.workflow::<pegboard::workflows::runner::Input>(conn.workflow_id)
			.get()
			.await?
		else {
			tracing::error!(?conn.runner_id, "workflow does not exist");
			continue;
		};

		// Check workflow is not dead
		if !wf.has_wake_condition {
			continue;
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
	}
}
