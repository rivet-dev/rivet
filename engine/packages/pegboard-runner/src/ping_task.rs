use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use rand::Rng;
use rivet_runner_protocol::{self as protocol, versioned};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use tokio::sync::watch;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn};

/// Updates the ping of all runners requesting a ping update at once.
#[tracing::instrument(name="ping_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
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

		update_runner_ping(&ctx, &conn).await?;

		// Send ping to runner
		if protocol::is_mk2(conn.protocol_version) {
			let ping_msg = versioned::ToClientMk2::wrap_latest(
				protocol::mk2::ToClient::ToClientPing(protocol::mk2::ToClientPing {
					ts: util::timestamp::now(),
				}),
			);
			let ping_msg_serialized = ping_msg.serialize(conn.protocol_version)?;
			conn.ws_handle
				.send(Message::Binary(ping_msg_serialized.into()))
				.await?;
		}
	}
}

async fn update_runner_ping(ctx: &StandaloneCtx, conn: &Conn) -> Result<()> {
	let wf = if protocol::is_mk2(conn.protocol_version) {
		ctx.workflow::<pegboard::workflows::runner2::Input>(conn.workflow_id)
			.get()
			.await?
	} else {
		ctx.workflow::<pegboard::workflows::runner::Input>(conn.workflow_id)
			.get()
			.await?
	};

	let Some(wf) = wf else {
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

	// If runner became eligible again, have the workflow check pending actor queue
	for notif in res.notifications {
		if let RunnerEligibility::ReEligible = notif.eligibility {
			tracing::debug!(runner_id=?notif.runner_id, "runner has become eligible again");

			if protocol::is_mk2(conn.protocol_version) {
				ctx.signal(pegboard::workflows::runner2::CheckQueue {})
					.to_workflow_id(notif.workflow_id)
					.send()
					.await?;
			} else {
				ctx.signal(pegboard::workflows::runner::CheckQueue {})
					.to_workflow_id(notif.workflow_id)
					.send()
					.await?;
			}
		}
	}

	Ok(())
}
