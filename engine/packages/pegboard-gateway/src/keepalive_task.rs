use anyhow::Result;
use gas::prelude::*;
use pegboard::tunnel::id as tunnel_id;
use pegboard::tunnel::id::{GatewayId, RequestId};
use rand::Rng;
use std::time::Duration;
use tokio::sync::watch;

use super::LifecycleResult;
use crate::shared_state::SharedState;

/// Periodically pings writes keepalive in UDB. This is used to restore hibernating request IDs on
/// next actor start.
///
/// Only ran for hibernating requests.
pub async fn task(
	shared_state: SharedState,
	ctx: StandaloneCtx,
	actor_id: Id,
	gateway_id: GatewayId,
	request_id: RequestId,
	mut keepalive_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut ping_interval = tokio::time::interval(Duration::from_millis(
		(ctx.config()
			.pegboard()
			.hibernating_request_eligible_threshold()
			/ 2)
		.try_into()?,
	));
	ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	loop {
		tokio::select! {
			_ = ping_interval.tick() => {}
			_ = keepalive_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		// Jitter sleep to prevent stampeding herds
		let jitter = { rand::thread_rng().gen_range(0..128) };
		tokio::time::sleep(Duration::from_millis(jitter)).await;

		tracing::debug!(
			%actor_id,
			gateway_id=%tunnel_id::gateway_id_to_string(&gateway_id),
			request_id=%tunnel_id::request_id_to_string(&request_id),
			"updating hws keepalive"
		);

		tokio::try_join!(
			ctx.op(pegboard::ops::actor::hibernating_request::upsert::Input {
				actor_id,
				gateway_id,
				request_id,
			}),
			// Keep alive in flight req during hibernation
			shared_state.keepalive_hws(request_id),
		)?;
	}
}
