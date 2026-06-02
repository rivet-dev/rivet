use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use rand::Rng;
use rivet_envoy_protocol::{self as protocol, versioned};
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use tokio::sync::watch;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, errors::WsError};

#[tracing::instrument(name="ping_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut ping_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let update_ping_interval =
		Duration::from_millis(ctx.config().pegboard().envoy_update_ping_interval());
	let ping_timeout_ms = ctx.config().pegboard().envoy_ping_timeout();

	loop {
		// Jitter sleep to prevent stampeding herds
		let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..128));
		tokio::select! {
			_ = tokio::time::sleep(update_ping_interval + jitter) => {}
			_ = ping_abort_rx.changed() => {
				return Ok(LifecycleResult::Aborted);
			}
		}

		// Check if the last ping is past the timeout threshold
		let last_ping_ts = conn.last_ping_ts.load(Ordering::SeqCst);
		let now = util::timestamp::now();
		let gap_ms = now - last_ping_ts;
		if gap_ms > ping_timeout_ms {
			tracing::warn!(
				envoy_key = %conn.envoy_key,
				last_ping_ts,
				now,
				gap_ms,
				threshold_ms = ping_timeout_ms,
				"envoy ping timed out, closing connection"
			);
			return Err(WsError::TimedOut.build());
		}

		// Update ping
		let last_rtt = conn.last_rtt.load(Ordering::Relaxed);
		ctx.op(pegboard::ops::envoy::update_ping::Input {
			namespace_id: conn.namespace_id,
			envoy_key: conn.envoy_key.clone(),
			update_lb: !conn.is_serverless,
			rtt: last_rtt,
		})
		.await?;

		tracing::debug!(
			gap_since_last_pong_ms = gap_ms,
			last_rtt_ms = last_rtt,
			"sending ping"
		);

		// Send ping to envoy
		let ping_msg = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyPing(
			protocol::ToEnvoyPing {
				ts: util::timestamp::now(),
			},
		));
		let ping_msg_serialized = ping_msg.serialize(conn.protocol_version)?;
		conn.ws_handle
			.send(Message::Binary(ping_msg_serialized.into()))
			.await?;
	}
}
